pub(crate) const TURNS: usize = 1_000_000;

pub(crate) async fn within_turns(ready: impl Fn() -> bool, missing: impl Fn() -> String) {
    within_turns_of(TURNS, ready, missing).await;
}

pub(crate) async fn within_turns_of(
    bound: usize,
    ready: impl Fn() -> bool,
    missing: impl Fn() -> String,
) {
    for _ in 0..bound {
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("after {bound} turns of the runtime: {}", missing());
}
