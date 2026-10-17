use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::acquire_tests::Rig;
use crate::claim::Claim;
use crate::h2state::H2State;
use crate::pool::{Plan, PoolKey, WaitKind};
use crate::settled::Settled;
use crate::transport::Conn;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Held {
    Charged,
    Free,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fresh {
    Dial,
    WaitsOnSame,
    Reuse { usable_now: bool },
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Awaiting,
    Zero,
    Usable,
    Full,
}

fn stage_of((held, fresh): (Held, Fresh)) -> Option<Stage> {
    match (held, fresh) {
        (Held::Free, Fresh::WaitsOnSame) => Some(Stage::Awaiting),
        (Held::Charged, Fresh::Dial) => Some(Stage::Zero),
        (Held::Free, Fresh::Reuse { usable_now: true }) => Some(Stage::Usable),
        (Held::Free, Fresh::Dial) => Some(Stage::Full),
        _ => None,
    }
}

struct Looker {
    rig: Rig,
    state: Arc<H2State>,
    key: PoolKey,
    pairs: Mutex<Vec<(Held, Fresh)>>,
    kept: Mutex<Vec<Conn>>,
}

impl Looker {
    fn held(&self) -> Held {
        let claim = Claim::Conn(Arc::clone(&self.state));
        match self.rig.shared.pool.plan(&self.key, Some(&claim)) {
            Plan::Settled(Settled::Spent) => Held::Charged,
            Plan::Settled(_) => Held::Other,
            Plan::Reuse(conn) => {
                self.kept.lock().expect("kept").push(conn);
                Held::Free
            }
            Plan::Wait(_) | Plan::Dial(_) | Plan::TryIdle(_) => Held::Free,
        }
    }

    fn fresh(&self) -> Fresh {
        match self.rig.shared.pool.plan(&self.key, None) {
            Plan::Dial(_) => Fresh::Dial,
            Plan::Wait(wait) => match wait.kind {
                WaitKind::Stream(Some(Claim::Conn(state))) if Arc::ptr_eq(&state, &self.state) => {
                    Fresh::WaitsOnSame
                }
                _ => Fresh::Other,
            },
            Plan::Reuse(conn) => {
                let usable_now = match &conn {
                    Conn::H2 { permit, .. } => {
                        let mut ready = std::pin::pin!(permit.usable_since_first_settings());
                        let mut cx = Context::from_waker(Waker::noop());
                        matches!(ready.as_mut().poll(&mut cx), Poll::Ready(true))
                    }
                    Conn::H1(_) => false,
                };
                self.kept.lock().expect("kept").push(conn);
                Fresh::Reuse { usable_now }
            }
            _ => Fresh::Other,
        }
    }

    fn look(&self) {
        let pair = (self.held(), self.fresh());
        self.pairs.lock().expect("pairs").push(pair);
    }
}

async fn stages(setup: &[&[Option<u32>]], run: &'static [Option<u32>]) -> Vec<Stage> {
    let rig = Rig::silent_server().await;
    let (state, held_by_dialer) = rig.land(rig.dialer()).await;
    let key = rig.key.clone();
    let looker = Arc::new(Looker {
        rig,
        state: Arc::clone(&state),
        key,
        pairs: Mutex::new(Vec::new()),
        kept: Mutex::new(Vec::new()),
    });
    for earlier in setup {
        state.apply_settings_run(earlier);
    }
    looker.rig.shared.pool.hooks.on_settings_step({
        let looker = Arc::downgrade(&looker);
        move || {
            if let Some(looker) = looker.upgrade() {
                looker.look();
            }
        }
    });
    state.apply_settings_run(run);
    looker.look();
    let pairs = looker.pairs.lock().expect("pairs").clone();
    drop(held_by_dialer);
    pairs
        .into_iter()
        .map(|pair| stage_of(pair).unwrap_or_else(|| panic!("an impossible look: {pair:?}")))
        .collect()
}

fn goes_from_to(seen: &[Stage], from: Stage, to: Stage) {
    assert!(seen.len() >= 4, "{seen:?}");
    assert_eq!(seen.last(), Some(&to), "{seen:?}");
    let mut arrived = false;
    for stage in seen {
        if *stage == to {
            arrived = true;
        } else {
            assert!(!arrived && *stage == from, "{seen:?}");
        }
    }
}

#[tokio::test]
async fn given_a_first_zero_when_a_waiter_looks_at_every_step_then_it_is_charged_once_and_never_waits_on_the_connection_again()
 {
    let seen = stages(&[], &[Some(0)]).await;
    goes_from_to(&seen, Stage::Awaiting, Stage::Zero);
}

#[tokio::test]
async fn given_a_first_zero_and_a_later_raise_when_sends_look_at_every_step_then_the_connection_is_usable_exactly_when_it_gives_a_permit()
 {
    let seen = stages(&[&[Some(0)]], &[Some(50)]).await;
    goes_from_to(&seen, Stage::Zero, Stage::Usable);
}

#[tokio::test]
async fn given_a_usable_first_settings_when_sends_look_at_every_step_then_a_permit_is_given_only_when_it_is_usable_at_once()
 {
    let seen = stages(&[], &[Some(100)]).await;
    goes_from_to(&seen, Stage::Awaiting, Stage::Usable);
}

#[tokio::test]
async fn given_a_first_zero_and_a_raise_in_the_same_run_when_sends_look_at_every_step_then_nobody_sees_the_zero()
 {
    let seen = stages(&[], &[Some(0), Some(100)]).await;
    goes_from_to(&seen, Stage::Awaiting, Stage::Usable);
}

#[tokio::test]
async fn given_a_usable_connection_and_a_later_zero_when_sends_look_at_every_step_then_nobody_is_charged()
 {
    let seen = stages(&[&[Some(100)]], &[Some(0)]).await;
    goes_from_to(&seen, Stage::Usable, Stage::Full);
}

#[tokio::test]
async fn given_a_run_of_settings_when_the_host_is_woken_then_what_the_server_announced_is_already_published()
 {
    let rig = Rig::silent_server().await;
    let (state, _held) = rig.land(rig.dialer()).await;
    let changes = rig
        .shared
        .pool
        .lock()
        .get(&rig.key)
        .expect("host")
        .signal
        .subscribe();
    let seen = Arc::new(Mutex::new(Vec::new()));
    rig.shared.pool.hooks.on_settings_step({
        let (state, seen) = (Arc::clone(&state), Arc::clone(&seen));
        move || {
            let woken = changes.has_changed().expect("open");
            seen.lock()
                .expect("seen")
                .push((state.awaiting_settings(), woken));
        }
    });
    state.apply_settings(Some(5));
    let seen = seen.lock().expect("seen").clone();
    assert!(
        seen.iter().all(|(awaiting, woken)| !(*woken && *awaiting)),
        "{seen:?}"
    );
    assert_eq!(seen.last(), Some(&(false, true)));
}
