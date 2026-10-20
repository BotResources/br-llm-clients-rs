use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::{Endpoint, HttpClient, HttpConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Captured {
    pub request_line: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Captured {
    pub fn header_values(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .collect()
    }
}

#[derive(Clone)]
pub enum Answer {
    Raw(Vec<u8>),
    Hang,
    Close,
}

pub fn response(status: u16, extra: &[(&str, &str)], body: &[u8]) -> Answer {
    let mut head = format!(
        "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    Answer::Raw(bytes)
}

pub fn json(status: u16, body: &str) -> Answer {
    response(
        status,
        &[("content-type", "application/json")],
        body.as_bytes(),
    )
}

pub fn short_body(status: u16, declared: usize, sent: &[u8]) -> Answer {
    let head =
        format!("HTTP/1.1 {status} X\r\ncontent-length: {declared}\r\nconnection: close\r\n\r\n");
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(sent);
    Answer::Raw(bytes)
}

pub struct Server {
    port: u16,
    captured: Arc<Mutex<Vec<Captured>>>,
    accept: JoinHandle<()>,
}

impl Server {
    pub async fn start(answers: Vec<Answer>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let seen = captured.clone();
        let counter = Arc::new(AtomicUsize::new(0));
        let accept = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let index = counter.fetch_add(1, Ordering::SeqCst);
                let answer = answers
                    .get(index)
                    .or_else(|| answers.last())
                    .cloned()
                    .expect("an answer");
                tokio::spawn(serve(stream, answer, seen.clone()));
            }
        });
        Self {
            port,
            captured,
            accept,
        }
    }

    pub fn endpoint(&self, path_and_query: &str) -> Endpoint {
        Endpoint::plain_http(&format!("http://127.0.0.1:{}{path_and_query}", self.port))
            .expect("endpoint")
    }

    pub fn captured(&self) -> Vec<Captured> {
        self.captured.lock().expect("captured").clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.accept.abort();
    }
}

pub fn http_client(headers_timeout: Duration) -> HttpClient {
    let config = HttpConfig::new(
        Duration::from_secs(5),
        headers_timeout,
        Duration::from_secs(5),
    );
    HttpClient::new(config).expect("client")
}

async fn serve(mut stream: TcpStream, answer: Answer, seen: Arc<Mutex<Vec<Captured>>>) {
    let Some(request) = read_request(&mut stream).await else {
        return;
    };
    seen.lock().expect("captured").push(request);
    match answer {
        Answer::Raw(bytes) => {
            let _ = stream.write_all(&bytes).await;
            let _ = stream.shutdown().await;
        }
        Answer::Hang => std::future::pending::<()>().await,
        Answer::Close => {}
    }
}

async fn read_request(stream: &mut TcpStream) -> Option<Captured> {
    let mut buffer: Vec<u8> = Vec::new();
    let split = loop {
        if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break at;
        }
        let mut chunk = [0u8; 4096];
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&buffer[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[split + 4..].to_vec();
    while body.len() < length {
        let mut chunk = [0u8; 4096];
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    Some(Captured {
        request_line,
        headers,
        body,
    })
}
