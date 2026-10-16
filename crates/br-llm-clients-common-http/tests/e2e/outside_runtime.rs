use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::{Context, Waker};

use br_llm_clients_common_http::{Endpoint, NoAuthorizer};

use crate::support::{client, post};

#[test]
fn given_a_send_polled_outside_a_tokio_runtime_then_it_panics() {
    let client = client();
    let endpoint = Endpoint::https("https://127.0.0.1:1/x").expect("endpoint");
    let polled = catch_unwind(AssertUnwindSafe(|| {
        let mut sending = Box::pin(client.send(post(&endpoint, "x"), &NoAuthorizer));
        let _ = sending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
    }));
    assert!(polled.is_err());
}
