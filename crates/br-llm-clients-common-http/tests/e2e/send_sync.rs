use br_llm_clients_common_http::{
    Authorizer, BodyStream, Collected, Endpoint, HttpClient, HttpConfig, Request, Response, Secret,
};

fn is_send<T: Send>() {}
fn is_send_sync<T: Send + Sync>() {}
fn is_clone<T: Clone>() {}
fn is_send_future<F: Send>(_: &F) {}

#[test]
fn given_the_public_types_then_they_have_the_marker_traits_the_intent_promises() {
    is_send_sync::<HttpClient>();
    is_clone::<HttpClient>();
    is_send_sync::<HttpConfig>();
    is_send_sync::<Endpoint>();
    is_send_sync::<Secret>();
    is_clone::<Secret>();
    is_send_sync::<Request>();
    is_send::<Response>();
    is_send::<BodyStream>();
    is_send_sync::<Collected>();
}

#[tokio::test]
async fn given_the_futures_of_the_library_then_they_can_move_between_threads() {
    let client = HttpClient::new(HttpConfig::new(
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(1),
    ))
    .expect("client");
    let endpoint = Endpoint::plain_http("http://127.0.0.1:1/").expect("endpoint");
    let authorizer: Box<dyn Authorizer> = Box::new(br_llm_clients_common_http::NoAuthorizer);
    let send = client.send(Request::new(endpoint, ""), authorizer.as_ref());
    is_send_future(&send);
    let moved = client.clone();
    let error = tokio::spawn(async move {
        moved
            .send(
                Request::new(
                    Endpoint::plain_http("http://127.0.0.1:1/").expect("endpoint"),
                    "",
                ),
                &br_llm_clients_common_http::NoAuthorizer,
            )
            .await
    })
    .await
    .expect("join");
    assert!(error.is_err());
    drop(send);
}

#[tokio::test]
async fn given_a_response_and_its_stream_then_they_can_be_moved_to_another_task() {
    let server = crate::support::canned::serve_response("200 OK", &[], b"moved").await;
    let response = crate::support::client()
        .send(
            crate::support::post(&server.endpoint("/"), "a"),
            &br_llm_clients_common_http::NoAuthorizer,
        )
        .await
        .expect("response");
    let body = tokio::spawn(async move {
        let mut body = response.into_body();
        body.collect(10).await.expect("body").bytes
    })
    .await
    .expect("join");
    assert_eq!(body, b"moved");
}

#[test]
fn given_the_dependency_tree_then_hyper_and_the_library_share_one_h2_so_error_downcasts_work() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"))
        .expect("Cargo.lock");
    let copies = lock.matches("name = \"h2\"").count();
    assert_eq!(
        copies, 1,
        "two copies of h2 would break the classification of HTTP/2 errors"
    );
}
