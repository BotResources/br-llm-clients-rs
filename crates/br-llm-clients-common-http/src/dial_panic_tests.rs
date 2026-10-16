use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::acquire::{DialOutcome, joined};
use crate::acquire_tests::{Outcome, Rig, outcome};
use crate::dialer::DialFailure;
use crate::send_error::SendError;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn given_a_dial_that_panics_then_the_sends_that_waited_for_it_share_one_connect_failure_and_none_dials_again()
 {
    let rig = Rig::announcing_server(100).await;
    let dials = Arc::new(AtomicUsize::new(0));
    let released = Arc::new(AtomicBool::new(false));
    rig.shared.pool.on_register({
        let (dials, released) = (Arc::clone(&dials), Arc::clone(&released));
        move || {
            dials.fetch_add(1, Ordering::SeqCst);
            while !released.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
            panic!("a fault injected into the dial");
        }
    });
    let dialing = rig.waiter();
    while !rig.dialing() {
        tokio::task::yield_now().await;
    }
    let mut sends = vec![dialing];
    sends.extend((0..5).map(|_| rig.waiter()));
    rig.waiting(5).await;
    released.store(true, Ordering::SeqCst);
    let mut texts = Vec::new();
    for send in sends {
        match outcome(send).await {
            Outcome::Failed(SendError::Connect { text }) => texts.push(text),
            other => panic!("every send fails with Connect: {}", describe(&other)),
        }
    }
    assert_eq!(texts.len(), 6);
    assert!(texts.iter().all(|text| *text == texts[0]), "{texts:?}");
    assert_eq!(dials.load(Ordering::SeqCst), 1);
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Spent => "spent".to_owned(),
        Outcome::Ready => "ready".to_owned(),
        Outcome::Failed(error) => format!("{error:?}"),
    }
}

#[tokio::test]
async fn given_a_dial_task_that_panicked_or_was_aborted_then_the_dial_failed_with_connect_and_its_fixed_text()
 {
    let panicked: JoinHandle<DialOutcome> = tokio::spawn(async { panic!("injected") });
    let aborted: JoinHandle<DialOutcome> = tokio::spawn(std::future::pending());
    aborted.abort();
    for result in [panicked.await, aborted.await] {
        let Err(DialFailure::Connect(text)) = joined(result) else {
            panic!("a lost dial is a Connect failure");
        };
        let DialFailure::Connect(expected) = DialFailure::lost() else {
            panic!("the lost dial is a Connect failure");
        };
        assert_eq!(text, expected);
    }
}

#[test]
fn given_a_dial_that_ended_then_the_join_changes_nothing() {
    assert!(matches!(joined(Ok(Ok(None))), Ok(None)));
    assert!(matches!(
        joined(Ok(Err(DialFailure::Tls("refused".to_owned())))),
        Err(DialFailure::Tls(text)) if text == "refused"
    ));
    assert!(matches!(
        joined(Ok(Err(DialFailure::Timeout))),
        Err(DialFailure::Timeout)
    ));
}
