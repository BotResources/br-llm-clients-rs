use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_auth::{AccessToken, TokenSource};
use br_llm_clients_common_http::{AuthorizerError, BoxFuture, Secret};
use tokio::sync::Notify;
use tokio::time::sleep;

#[derive(Clone)]
pub enum Then {
    Token(&'static str, Option<Duration>),
    Fail(AuthorizerError),
    Hang,
    Panic,
}

#[derive(Clone)]
pub struct Step {
    pub then: Then,
    pub delay: Duration,
    pub gate: Option<Arc<Notify>>,
}

impl Step {
    pub fn token(text: &'static str, valid_for: Option<Duration>) -> Self {
        Self::of(Then::Token(text, valid_for))
    }

    pub fn secs(text: &'static str, seconds: u64) -> Self {
        Self::token(text, Some(Duration::from_secs(seconds)))
    }

    pub fn fail(error: AuthorizerError) -> Self {
        Self::of(Then::Fail(error))
    }

    pub fn hang() -> Self {
        Self::of(Then::Hang)
    }

    pub fn panic() -> Self {
        Self::of(Then::Panic)
    }

    pub fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn gated(mut self, gate: &Arc<Notify>) -> Self {
        self.gate = Some(gate.clone());
        self
    }

    fn of(then: Then) -> Self {
        Self {
            then,
            delay: Duration::ZERO,
            gate: None,
        }
    }
}

pub struct Scripted {
    steps: Vec<Step>,
    calls: AtomicUsize,
    held: Mutex<Vec<Secret>>,
}

impl Scripted {
    pub fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            steps,
            calls: AtomicUsize::new(0),
            held: Mutex::new(Vec::new()),
        })
    }

    pub fn holding(steps: Vec<Step>, held: Vec<Secret>) -> Arc<Self> {
        Arc::new(Self {
            steps,
            calls: AtomicUsize::new(0),
            held: Mutex::new(held),
        })
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl TokenSource for Scripted {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let step = self
            .steps
            .get(index)
            .or_else(|| self.steps.last())
            .cloned()
            .expect("a scripted source needs a step");
        Box::pin(async move {
            if let Some(gate) = &step.gate {
                gate.notified().await;
            }
            if !step.delay.is_zero() {
                sleep(step.delay).await;
            }
            match step.then {
                Then::Token(text, valid_for) => Ok(AccessToken::new(
                    Secret::new(text).expect("secret"),
                    valid_for,
                )),
                Then::Fail(error) => Err(error),
                Then::Hang => std::future::pending().await,
                Then::Panic => panic!("the scripted source panics"),
            }
        })
    }

    fn secrets(&self) -> Vec<Secret> {
        self.held.lock().expect("held").clone()
    }
}
