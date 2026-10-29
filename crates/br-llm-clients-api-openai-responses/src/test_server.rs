use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use br_llm_clients_common_call::Endpoint;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone)]
pub(crate) struct Received {
    pub(crate) request_line: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Received {
    pub(crate) fn header_values(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .collect()
    }

    pub(crate) fn body_json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).expect("the request body is JSON")
    }
}

#[derive(Clone)]
pub(crate) struct Wire {
    pub(crate) bytes: Vec<u8>,
    head_len: usize,
    cut: Option<usize>,
}

impl Wire {
    pub(crate) fn split_body_at(mut self, offset: usize) -> Self {
        self.cut = Some(self.head_len.saturating_add(offset));
        self
    }
}

pub(crate) fn http(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Wire {
    let mut bytes = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "content-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    let head_len = bytes.len();
    bytes.extend_from_slice(body);
    Wire {
        bytes,
        head_len,
        cut: None,
    }
}

pub(crate) struct TestServer {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<Received>>>,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub(crate) async fn start(reply: impl Fn(usize) -> Wire + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        let reply = Arc::new(reply);
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(stream, Arc::clone(&log), Arc::clone(&reply)));
            }
        });
        Self {
            addr,
            requests,
            task,
        }
    }

    pub(crate) fn requests(&self) -> Vec<Received> {
        self.requests.lock().expect("log").clone()
    }

    pub(crate) fn endpoint(&self, path: &str) -> Endpoint {
        Endpoint::plain_http(&format!("http://{}{}", self.addr, path)).expect("endpoint")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(
    mut stream: TcpStream,
    log: Arc<Mutex<Vec<Received>>>,
    reply: Arc<impl Fn(usize) -> Wire>,
) {
    let Ok(received) = read_request(&mut stream).await else {
        return;
    };
    let index = {
        let mut log = log.lock().expect("log");
        log.push(received);
        log.len().saturating_sub(1)
    };
    let wire = reply(index);
    match wire.cut {
        Some(cut) if cut < wire.bytes.len() => {
            let (first, second) = wire.bytes.split_at(cut);
            let _ = stream.write_all(first).await;
            let _ = stream.flush().await;
            tokio::task::yield_now().await;
            let _ = stream.write_all(second).await;
        }
        _ => {
            let _ = stream.write_all(&wire.bytes).await;
        }
    }
    let _ = stream.flush().await;
    let _ = stream.shutdown().await;
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<Received> {
    let mut buffer: Vec<u8> = Vec::new();
    let head_end = loop {
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }
        let mut chunk = [0u8; 4096];
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < length {
        let mut chunk = vec![0u8; length - body.len()];
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        body.extend_from_slice(&chunk[..read]);
    }
    Ok(Received {
        request_line,
        headers,
        body,
    })
}
