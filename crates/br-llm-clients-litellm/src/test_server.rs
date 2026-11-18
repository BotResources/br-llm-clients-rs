use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Endpoint;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub(crate) struct Reply {
    pub(crate) status: &'static str,
    pub(crate) headers: Vec<(&'static str, &'static str)>,
    pub(crate) body: Vec<u8>,
}

impl Reply {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = format!("HTTP/1.1 {}\r\n", self.status).into_bytes();
        for (name, value) in &self.headers {
            bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "content-length: {}\r\nconnection: close\r\n\r\n",
                self.body.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

type HeaderList = Vec<(String, String)>;

pub(crate) struct TestServer {
    base: Endpoint,
    received: Arc<Mutex<Vec<HeaderList>>>,
}

impl TestServer {
    pub(crate) async fn start(reply: Reply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&received);
        let wire = Arc::new(reply.bytes());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let log = Arc::clone(&log);
                let wire = Arc::clone(&wire);
                tokio::spawn(async move {
                    let headers = serve(stream, &wire).await;
                    log.lock().unwrap().push(headers);
                });
            }
        });
        let base = Endpoint::plain_http(&format!("http://{address}")).unwrap();
        Self { base, received }
    }

    pub(crate) fn base(&self) -> Endpoint {
        self.base.clone()
    }

    pub(crate) fn header_values(&self, name: &str) -> Vec<String> {
        let received = self.received.lock().unwrap();
        assert_eq!(received.len(), 1, "one request is received");
        received
            .iter()
            .flatten()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
            .collect()
    }

    pub(crate) fn values_per_request(&self, name: &str) -> Vec<Vec<String>> {
        self.received
            .lock()
            .unwrap()
            .iter()
            .map(|headers| {
                headers
                    .iter()
                    .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value.clone())
                    .collect()
            })
            .collect()
    }
}

async fn serve(mut stream: TcpStream, wire: &[u8]) -> Vec<(String, String)> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let head_end = loop {
        if let Some(end) = find(&buffer, b"\r\n\r\n") {
            break end;
        }
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "the request head is complete");
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8(buffer[..head_end].to_vec()).unwrap();
    let headers: Vec<(String, String)> = head
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.parse().unwrap())
        .unwrap_or(0);
    let mut body = buffer.len() - head_end - 4;
    while body < length {
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "the request body is complete");
        body += read;
    }
    stream.write_all(wire).await.unwrap();
    stream.shutdown().await.unwrap();
    headers
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
