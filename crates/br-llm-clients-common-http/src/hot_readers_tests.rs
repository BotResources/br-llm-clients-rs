use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::h2state::H2State;

const LOOKS_BEFORE_THE_WRITE: usize = 25;

pub(crate) fn while_looked_at(
    states: &[Arc<H2State>],
    readers: usize,
    look: impl Fn(&Arc<H2State>) + Sync,
    write: impl Fn(&Arc<H2State>),
) -> usize {
    let current = AtomicUsize::new(0);
    let looks = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..readers {
            scope.spawn(|| {
                while let Some(state) = states.get(current.load(Ordering::SeqCst)) {
                    look(state);
                    looks.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
        for (index, state) in states.iter().enumerate() {
            current.store(index, Ordering::SeqCst);
            let before = looks.load(Ordering::SeqCst);
            while looks.load(Ordering::SeqCst) < before.saturating_add(LOOKS_BEFORE_THE_WRITE) {
                std::thread::yield_now();
            }
            write(state);
        }
        current.store(states.len(), Ordering::SeqCst);
    });
    looks.load(Ordering::SeqCst)
}
