use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::h2state::H2State;
use crate::h2state_tests::bare_state;
use crate::hot_readers::while_looked_at;

const LONG_ENOUGH: Duration = Duration::from_secs(20);

fn states() -> Vec<Arc<H2State>> {
    (0..3).map(|_| bare_state()).collect()
}

fn ends_in_a_panic_in_time(work: impl FnOnce() + Send + 'static) -> bool {
    let (told, ended) = mpsc::channel();
    std::thread::spawn(move || {
        let panicked = catch_unwind(AssertUnwindSafe(work)).is_err();
        let _ = told.send(panicked);
    });
    ended
        .recv_timeout(LONG_ENOUGH)
        .expect("the probe hangs instead of ending")
}

#[test]
fn given_a_write_that_panics_then_the_readers_stop_and_the_panic_comes_out() {
    let panicked = ends_in_a_panic_in_time(|| {
        let states = states();
        while_looked_at(&states, 2, |_| {}, |_| panic!("the write failed"));
    });
    assert!(panicked);
}

#[test]
fn given_readers_that_all_panic_then_the_writer_stops_waiting_for_looks_and_the_panic_comes_out() {
    let panicked = ends_in_a_panic_in_time(|| {
        let states = states();
        while_looked_at(&states, 2, |_| panic!("the look failed"), |_| {});
    });
    assert!(panicked);
}

#[test]
fn given_readers_and_writes_then_every_state_is_written_once_after_enough_looks() {
    let states = states();
    let writes = AtomicUsize::new(0);
    let looks = while_looked_at(
        &states,
        2,
        |_| {},
        |_| {
            writes.fetch_add(1, Ordering::SeqCst);
        },
    );
    assert_eq!(writes.load(Ordering::SeqCst), states.len());
    assert!(looks >= 25 * states.len());
}
