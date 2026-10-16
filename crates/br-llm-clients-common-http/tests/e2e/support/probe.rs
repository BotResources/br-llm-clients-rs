use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::Endpoint;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use super::pki::Pki;
use super::probe_serve::serve;
use super::probe_types::Records;
pub use super::probe_types::{Arrived, Script};

pub struct Probe {
    pub addr: SocketAddr,
    pub accepted: Arc<AtomicUsize>,
    records: Records,
    task: tokio::task::JoinHandle<()>,
}

impl Probe {
    pub async fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let records = Records::new();
        let acceptor = Pki::trusted().acceptor(&[b"h2"]);
        let (count, shared) = (Arc::clone(&accepted), records.clone());
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let (acceptor, records) = (acceptor.clone(), shared.clone());
                connections.spawn(async move {
                    if let Ok(tls) = acceptor.accept(tcp).await {
                        serve(script, index, tls, records).await;
                    }
                });
            }
        });
        Self {
            addr,
            accepted,
            records,
            task,
        }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::https(&format!("https://127.0.0.1:{}/x", self.addr.port())).expect("endpoint")
    }

    pub fn arrived(&self) -> Vec<Arrived> {
        self.records.arrived()
    }

    pub fn arrivals(&self) -> Vec<String> {
        self.records.arrivals()
    }

    pub async fn wait_arrivals(&self, count: usize) {
        self.records.wait_arrivals(count).await;
    }

    pub fn processed(&self) -> Vec<String> {
        self.records.processed()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.task.abort();
    }
}
