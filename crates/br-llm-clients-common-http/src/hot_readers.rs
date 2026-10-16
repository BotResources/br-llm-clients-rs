use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::h2state::H2State;

pub(crate) const LOOKS_BEFORE_THE_WRITE: usize = 25;
const TIME_FOR_THE_LOOKS: Duration = Duration::from_secs(60);

struct Release<'a> {
    current: &'a AtomicUsize,
    len: usize,
}

impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.current.store(self.len, Ordering::SeqCst);
    }
}

struct Reading<'a>(&'a AtomicUsize);

impl Drop for Reading<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) fn while_looked_at(
    states: &[Arc<H2State>],
    readers: usize,
    look: impl Fn(&Arc<H2State>) + Sync,
    write: impl Fn(&Arc<H2State>),
) -> usize {
    let current = AtomicUsize::new(0);
    let looks = AtomicUsize::new(0);
    let reading = AtomicUsize::new(readers);
    std::thread::scope(|scope| {
        let _release = Release {
            current: &current,
            len: states.len(),
        };
        for _ in 0..readers {
            scope.spawn(|| {
                let _reading = Reading(&reading);
                while let Some(state) = states.get(current.load(Ordering::SeqCst)) {
                    look(state);
                    looks.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
        for (index, state) in states.iter().enumerate() {
            let deadline = Instant::now() + TIME_FOR_THE_LOOKS;
            current.store(index, Ordering::SeqCst);
            let before = looks.load(Ordering::SeqCst);
            while looks.load(Ordering::SeqCst) < before.saturating_add(LOOKS_BEFORE_THE_WRITE) {
                assert!(
                    reading.load(Ordering::SeqCst) > 0,
                    "every reader ended before it looked enough"
                );
                assert!(
                    Instant::now() < deadline,
                    "the readers did not look enough within {TIME_FOR_THE_LOOKS:?}"
                );
                std::thread::yield_now();
            }
            write(state);
        }
    });
    looks.load(Ordering::SeqCst)
}
