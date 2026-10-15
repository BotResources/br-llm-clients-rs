use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::timeout;

use crate::unsent::never_taken;

const BRIEF: Duration = Duration::from_millis(150);

#[tokio::test]
async fn given_a_connection_that_ended_and_a_request_never_written_then_it_counts_as_never_taken() {
    let (tx, mut gone) = watch::channel(false);
    let taken = Arc::new(AtomicBool::new(false));
    tx.send_replace(true);
    assert!(timeout(BRIEF, never_taken(&mut gone, &taken)).await.is_ok());
}

#[tokio::test]
async fn given_a_connection_that_ended_after_the_request_was_written_then_it_is_not_never_taken() {
    let (tx, mut gone) = watch::channel(false);
    let taken = Arc::new(AtomicBool::new(true));
    tx.send_replace(true);
    assert!(
        timeout(BRIEF, never_taken(&mut gone, &taken))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn given_a_connection_that_is_alive_then_the_request_is_not_never_taken() {
    let (_tx, mut gone) = watch::channel(false);
    let taken = Arc::new(AtomicBool::new(false));
    assert!(
        timeout(BRIEF, never_taken(&mut gone, &taken))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn given_a_request_written_while_the_connection_ends_then_a_later_end_is_still_not_never_taken()
 {
    let (tx, mut gone) = watch::channel(false);
    let taken = Arc::new(AtomicBool::new(false));
    let marker = Arc::clone(&taken);
    let ending = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        marker.store(true, Ordering::SeqCst);
        tx.send_replace(true);
    });
    assert!(
        timeout(BRIEF, never_taken(&mut gone, &taken))
            .await
            .is_err()
    );
    ending.await.expect("join");
}
