use std::net::SocketAddr;

use br_llm_clients_common_http::Endpoint;
use bytes::Bytes;
use tokio::net::TcpListener;

use super::pki::Pki;

pub struct HeadersServer {
    pub addr: SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl HeadersServer {
    pub async fn start(headers: Vec<(String, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let task = tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let (acceptor, headers) = (acceptor.clone(), headers.clone());
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let Ok(mut conn) = h2::server::Builder::new()
                        .max_header_list_size(1 << 24)
                        .handshake::<_, Bytes>(tls)
                        .await
                    else {
                        return;
                    };
                    while let Some(Ok((_request, mut respond))) = conn.accept().await {
                        let mut builder = http::Response::builder().status(200);
                        for (name, value) in &headers {
                            builder = builder.header(name.as_str(), value.as_str());
                        }
                        let response = builder.body(()).expect("response");
                        let _ = respond.send_response(response, true);
                    }
                });
            }
        });
        Self { addr, task }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }
}

impl Drop for HeadersServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
