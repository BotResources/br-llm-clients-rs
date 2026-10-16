use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError, BoxFuture, Secret,
    SendError,
};

use crate::resend::{client, identified};
use crate::support::guarded;
use crate::support::probe::{Arrived, Probe, Script};
use crate::support::spy::Spy;

struct Numbering(AtomicUsize);

impl Authorizer for Numbering {
    fn authorize<'a>(
        &'a self,
        _: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        let call = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        let value = Secret::new(format!("key-{call}")).expect("secret");
        Box::pin(async move { Ok(vec![AuthorizedHeader::new("x-api-key", value)]) })
    }
}

#[tokio::test]
async fn given_a_refused_stream_when_sent_again_then_the_second_arrival_carries_the_bytes_authorized_the_first_time()
 {
    let probe = Probe::start(Script::RefuseFirst(1)).await;
    let authorizer = Numbering(AtomicUsize::new(0));
    let mut response = guarded(client().send(identified(&probe, "r1"), &authorizer))
        .await
        .expect("sent again");
    response.body_mut().collect(10).await.expect("body");
    let arrived = Arrived {
        id: "r1".to_owned(),
        key: Some("key-1".to_owned()),
        body: Some(b"payload".to_vec()),
    };
    assert_eq!(probe.arrived(), [arrived.clone(), arrived]);
    assert_eq!(authorizer.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_a_stream_reset_asking_for_http_1_1_then_the_send_fails_with_protocol_and_is_not_sent_again()
 {
    let probe = Probe::start(Script::ResetAll(h2::Reason::HTTP_1_1_REQUIRED)).await;
    let error = guarded(client().send(identified(&probe, "r1"), &Spy::returning(&[])))
        .await
        .unwrap_err();
    assert!(matches!(error, SendError::Protocol { .. }), "{error:?}");
    assert_eq!(probe.arrivals(), ["r1"]);
    assert_eq!(probe.accepted.load(Ordering::SeqCst), 1);
}
