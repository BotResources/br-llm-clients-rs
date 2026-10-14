use br_llm_clients_common_http::{ApiKeyAuthorizer, Secret};
use tokio::io::AsyncWriteExt;

use crate::support::raw::{RawServer, read_request, write_response};
use crate::support::{client, post};

#[tokio::test]
async fn smoke_plain_http_roundtrip() {
    let server = RawServer::start(|mut stream, _| async move {
        let received = read_request(&mut stream).await.expect("request");
        println!("{received:?}");
        write_response(
            &mut stream,
            "200 OK",
            &[("x-a", "1"), ("x-a", "2")],
            b"hello",
        )
        .await
        .expect("response");
        let _ = stream.shutdown().await;
    })
    .await;
    let endpoint = server.endpoint("/v1/x?a=b");
    let auth = ApiKeyAuthorizer::new("x-api-key", Secret::new("sk-1").unwrap()).unwrap();
    let mut response = client()
        .send(post(&endpoint, "{}"), &auth)
        .await
        .expect("response");
    assert_eq!(response.status(), 200);
    let collected = response.body_mut().collect(100).await.expect("body");
    assert_eq!(collected.bytes, b"hello");
    assert!(!collected.cut);
}
