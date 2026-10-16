use std::time::Duration;

use crate::support::gate_kit::{answering, options, run, trusting};
use crate::support::guarded;
use crate::support::h2_server::TlsServer;
use crate::support::pki::Pki;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn given_a_burst_larger_than_the_server_limit_then_every_send_succeeds_without_queueing_in_a_connection()
 {
    let server = TlsServer::start_with(
        Pki::trusted().acceptor(&[b"h2"]),
        answering(Duration::from_millis(50)),
        options(100, Duration::ZERO, 0),
    )
    .await;
    let client = trusting(Duration::from_secs(20), Duration::from_secs(5));
    let outcomes = guarded(run(&client, &server, 300)).await;
    assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    let connections_allowed = 300_usize.div_ceil(100) + 1;
    assert!(
        server.accepted() <= connections_allowed,
        "{} connections",
        server.accepted()
    );
}
