pub(crate) const TURNS: usize = 1_000_000;

pub(crate) async fn within_turns(ready: impl Fn() -> bool, missing: impl Fn() -> String) {
    for _ in 0..TURNS {
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("after {TURNS} turns of the runtime: {}", missing());
}

pub(crate) fn within_spins(ready: impl Fn() -> bool, missing: impl Fn() -> String) {
    for _ in 0..TURNS {
        if ready() {
            return;
        }
        std::thread::yield_now();
    }
    panic!("after {TURNS} spins of the thread: {}", missing());
}
