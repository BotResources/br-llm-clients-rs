use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::task::{RawWaker, RawWakerVTable, Wake, Waker};
use std::time::Duration;

use br_llm_clients_core::Client;
use br_llm_messages::{DeltaOp, ResponseMetadata};

use crate::call_log::StateKind;
use crate::gate::Gate;
use crate::mock_client::MockClient;
use crate::recording_sink::RecordingSink;
use crate::reply::Reply;
use crate::test_support::*;

pub(crate) type Probe = Arc<dyn Fn() + Send + Sync>;

struct Probing {
    probe: Probe,
}

impl Wake for Probing {
    fn wake(self: Arc<Self>) {
        drop(self);
    }
}

impl Drop for Probing {
    fn drop(&mut self) {
        (self.probe)();
    }
}

struct Panicking;

impl Wake for Panicking {
    fn wake(self: Arc<Self>) {
        panic!("the waker failed");
    }

    fn wake_by_ref(self: &Arc<Self>) {
        panic!("the waker failed");
    }
}

pub(crate) fn probing_waker(probe: Probe) -> Waker {
    Waker::from(Arc::new(Probing { probe }))
}

pub(crate) fn panicking_waker() -> Waker {
    Waker::from(Arc::new(Panicking))
}

static CLONING: RawWakerVTable =
    RawWakerVTable::new(clone_hook, release_hook, ignore_hook, release_hook);

fn clone_hook(data: *const ()) -> RawWaker {
    let data = data.cast::<Probe>();
    // SAFETY: data comes from Arc::into_raw in cloning_waker and the strong count is at least one while a waker holds it.
    let probe = unsafe {
        Arc::increment_strong_count(data);
        Arc::from_raw(data)
    };
    (*probe)();
    RawWaker::new(Arc::into_raw(probe).cast::<()>(), &CLONING)
}

fn release_hook(data: *const ()) {
    // SAFETY: data comes from Arc::into_raw in cloning_waker and each waker releases exactly one strong count.
    unsafe { Arc::decrement_strong_count(data.cast::<Probe>()) };
}

fn ignore_hook(_data: *const ()) {}

pub(crate) fn cloning_waker(probe: Probe) -> Waker {
    let data = Arc::into_raw(Arc::new(probe)).cast::<()>();
    // SAFETY: the vtable hooks keep the Arc strong count equal to the number of live wakers made from data.
    unsafe { Waker::from_raw(RawWaker::new(data, &CLONING)) }
}

pub(crate) fn within_deadline(work: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        work();
        let _ = done.send(());
    });
    finished
        .recv_timeout(Duration::from_secs(10))
        .expect("the work deadlocked or panicked");
}

pub(crate) struct Subject {
    pub(crate) name: &'static str,
    pub(crate) future: Pin<Box<dyn Future<Output = ()> + Send>>,
    pub(crate) fire: Probe,
    pub(crate) touch: Probe,
}

fn complete_one_call(client: &MockClient) {
    let mut sink = RecordingSink::new();
    let _ = block_on(client.call(&request(), &mut sink));
}

fn answering_mock() -> MockClient {
    let client = mock();
    client.set_fallback(Reply::success(text_response("ok")).build().unwrap());
    client
}

fn gate_pass() -> Subject {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(
        Reply::success(text_response("ok"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let (opening, probing) = (gate.clone(), gate);
    Subject {
        name: "gate pass",
        future: Box::pin(async move {
            let mut sink = RecordingSink::new();
            let _ = client.call(&request(), &mut sink).await;
        }),
        fire: Arc::new(move || opening.open()),
        touch: Arc::new(move || {
            let _ = probing.waiting();
        }),
    }
}

fn wait_for_waiting() -> Subject {
    let gate = Gate::new();
    let client = mock();
    client.set_fallback(
        Reply::success(text_response("ok"))
            .gate_at(0, &gate)
            .build()
            .unwrap(),
    );
    let (waiting, probing) = (gate.clone(), gate.clone());
    let held = Arc::new(Mutex::new(Vec::new()));
    Subject {
        name: "wait for waiting",
        future: Box::pin(waiting.wait_for_waiting(1)),
        fire: Arc::new(move || {
            let client = client.clone();
            let mut call = Box::pin(async move {
                let mut sink = RecordingSink::new();
                let _ = client.call(&request(), &mut sink).await;
            });
            assert!(poll_once(&mut call.as_mut()).is_pending());
            held.lock().unwrap().push(call);
        }),
        touch: Arc::new(move || {
            let _ = probing.waiting();
        }),
    }
}

fn wait_for_calls() -> Subject {
    let client = answering_mock();
    let (firing, probing) = (client.clone(), client.clone());
    Subject {
        name: "wait for calls",
        future: Box::pin(client.wait_for_calls(1)),
        fire: Arc::new(move || complete_one_call(&firing)),
        touch: Arc::new(move || {
            let _ = probing.calls();
        }),
    }
}

fn wait_for_state() -> Subject {
    let client = answering_mock();
    let (firing, probing) = (client.clone(), client.clone());
    Subject {
        name: "wait for state",
        future: Box::pin(client.wait_for_state(StateKind::Succeeded, 1).unwrap()),
        fire: Arc::new(move || complete_one_call(&firing)),
        touch: Arc::new(move || {
            let _ = probing.log();
        }),
    }
}

fn wait_for_deltas() -> Subject {
    let sink = RecordingSink::new();
    let (firing, probing) = (sink.clone(), sink.clone());
    Subject {
        name: "wait for deltas",
        future: Box::pin(sink.wait_for_deltas(1)),
        fire: Arc::new(move || {
            let mut sink = firing.clone();
            block_on(br_llm_clients_core::StreamSink::send(
                &mut sink,
                delta(DeltaOp::response_started(ResponseMetadata::new())),
            ));
        }),
        touch: Arc::new(move || {
            let _ = probing.len();
        }),
    }
}

pub(crate) fn subjects() -> Vec<Subject> {
    vec![
        gate_pass(),
        wait_for_waiting(),
        wait_for_calls(),
        wait_for_state(),
        wait_for_deltas(),
    ]
}
