use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use br_llm_clients_common_http::{HttpClient, NoAuthorizer, Response, SendError};

use crate::support::frames::response_head;
use crate::support::gate_kit::trusting;
use crate::support::post;
use crate::support::raw_h2::{Announce, Answer, Policy, RawH2};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const HEADERS_TIMEOUT: Duration = Duration::from_secs(2);

type Started = Pin<Box<dyn Future<Output = Result<Response, SendError>> + Send>>;

fn started_send(client: &HttpClient, server: &RawH2) -> Started {
    let (client, endpoint) = (client.clone(), server.endpoint());
    Box::pin(async move { client.send(post(&endpoint, "stalled"), &NoAuthorizer).await })
}

async fn polled_once(send: &mut Started) {
    poll_fn(|context| {
        assert!(send.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

async fn three_sends_in_turn(client: &HttpClient, server: &RawH2) {
    for turn in 0..3 {
        let mut response = client
            .send(post(&server.endpoint(), "x"), &NoAuthorizer)
            .await
            .unwrap_or_else(|error| panic!("send {turn}: {error:?}"));
        response.body_mut().collect(10).await.expect("body");
    }
}

fn holding_first_connection(index: usize) -> Policy {
    let held = Policy::announcing(Announce::Limit(1));
    if index == 0 {
        Policy {
            answer: Answer::Frames(|stream| response_head(stream, None, false)),
            ..held
        }
    } else {
        held
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_cold_host_and_a_send_that_started_a_dial_and_is_not_polled_then_the_other_sends_are_served_on_that_dial()
 {
    let server = RawH2::start(|_| Policy::healthy()).await;
    let client = trusting(CONNECT_TIMEOUT, HEADERS_TIMEOUT);
    let mut stalled = started_send(&client, &server);
    polled_once(&mut stalled).await;
    three_sends_in_turn(&client, &server).await;
    assert_eq!(server.accepted(), 1);
    drop(stalled);
    three_sends_in_turn(&client, &server).await;
    assert_eq!(server.accepted(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_cold_host_and_a_send_that_started_a_dial_and_is_polled_again_later_then_it_completes()
 {
    let server = RawH2::start(|_| Policy::healthy()).await;
    let client = trusting(CONNECT_TIMEOUT, HEADERS_TIMEOUT);
    let mut stalled = started_send(&client, &server);
    polled_once(&mut stalled).await;
    three_sends_in_turn(&client, &server).await;
    let mut response = stalled.await.expect("the stalled send completes");
    response.body_mut().collect(10).await.expect("body");
    assert_eq!(server.accepted(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_busy_host_and_a_send_that_started_a_dial_for_one_more_connection_and_is_not_polled_then_the_other_sends_are_served()
 {
    let server = RawH2::start(holding_first_connection).await;
    let client = trusting(CONNECT_TIMEOUT, HEADERS_TIMEOUT);
    let held = client
        .send(post(&server.endpoint(), "held"), &NoAuthorizer)
        .await
        .expect("the first connection holds a stream");
    let mut stalled = started_send(&client, &server);
    polled_once(&mut stalled).await;
    three_sends_in_turn(&client, &server).await;
    assert_eq!(server.accepted(), 3);
    drop(stalled);
    three_sends_in_turn(&client, &server).await;
    assert_eq!(server.accepted(), 3);
    drop(held);
}

fn holding_every_stream() -> Policy {
    Policy {
        answer: Answer::Frames(|stream| response_head(stream, None, false)),
        ..Policy::announcing(Announce::Limit(1))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_send_that_started_a_dial_and_is_not_polled_again_when_the_connection_allows_one_stream_then_it_stays_full_until_the_send_is_dropped()
 {
    let server = RawH2::start(|_| holding_every_stream()).await;
    let client = trusting(CONNECT_TIMEOUT, HEADERS_TIMEOUT);
    let mut stalled = started_send(&client, &server);
    polled_once(&mut stalled).await;
    while server.accepted() == 0 {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let beside = client
        .send(post(&server.endpoint(), "beside"), &NoAuthorizer)
        .await
        .expect("a send beside the stalled one");
    assert_eq!(server.accepted(), 2);
    drop(stalled);
    let after = client
        .send(post(&server.endpoint(), "after"), &NoAuthorizer)
        .await
        .expect("a send after the stalled one was dropped");
    assert_eq!(server.accepted(), 2);
    drop((beside, after));
}
