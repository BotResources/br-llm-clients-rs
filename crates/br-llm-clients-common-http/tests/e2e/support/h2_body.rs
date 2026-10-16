use std::net::SocketAddr;
use std::time::Duration;

use br_llm_clients_common_http::Endpoint;
use bytes::Bytes;
use h2::Reason;
use tokio::net::TcpListener;

use super::pki::Pki;

#[derive(Clone, Copy)]
pub enum End {
    Fin,
    ResetNoError,
}

#[derive(Clone, Copy)]
pub struct BodyScript {
    pub declared: Option<usize>,
    pub frames: &'static [&'static [u8]],
    pub end: End,
}

pub struct BodyServer {
    pub addr: SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl BodyServer {
    pub async fn start(script: BodyScript) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let task = tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let Ok(mut conn) = h2::server::handshake(tls).await else {
                        return;
                    };
                    while let Some(Ok((_request, respond))) = conn.accept().await {
                        tokio::spawn(answer(respond, script));
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

impl Drop for BodyServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn answer(mut respond: h2::server::SendResponse<Bytes>, script: BodyScript) {
    let mut builder = http::Response::builder().status(200);
    if let Some(declared) = script.declared {
        builder = builder.header("content-length", declared.to_string());
    }
    let response = builder.body(()).expect("response");
    let Ok(mut stream) = respond.send_response(response, false) else {
        return;
    };
    for frame in script.frames {
        if stream.send_data(Bytes::from_static(frame), false).is_err() {
            return;
        }
    }
    match script.end {
        End::Fin => {
            let _ = stream.send_data(Bytes::new(), true);
        }
        End::ResetNoError => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            stream.send_reset(Reason::NO_ERROR);
        }
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
}
