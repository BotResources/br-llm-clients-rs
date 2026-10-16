use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct Proxy {
    pub addr: SocketAddr,
    pub accepted: Arc<AtomicUsize>,
    pub blackholed: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Proxy {
    pub async fn start(upstream: SocketAddr) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let accepted = Arc::new(AtomicUsize::new(0));
        let blackholed = Arc::new(AtomicUsize::new(usize::MAX));
        let (count, hole) = (Arc::clone(&accepted), Arc::clone(&blackholed));
        let task = tokio::spawn(async move {
            loop {
                let Ok((down, _)) = listener.accept().await else {
                    break;
                };
                let index = count.fetch_add(1, Ordering::SeqCst);
                let hole = Arc::clone(&hole);
                tokio::spawn(async move {
                    let Ok(up) = TcpStream::connect(upstream).await else {
                        return;
                    };
                    let (down_read, down_write) = down.into_split();
                    let (up_read, up_write) = up.into_split();
                    let forward_up = pump(down_read, up_write, index, Arc::clone(&hole));
                    let forward_down = pump(up_read, down_write, index, hole);
                    let _ = tokio::join!(forward_up, forward_down);
                });
            }
        });
        Self {
            addr,
            accepted,
            blackholed,
            task,
        }
    }

    pub fn blackhole(&self, connection: usize) {
        self.blackholed.store(connection, Ordering::SeqCst);
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn pump(
    mut from: impl AsyncReadExt + Unpin,
    mut to: impl AsyncWriteExt + Unpin,
    index: usize,
    hole: Arc<AtomicUsize>,
) {
    let mut buffer = vec![0u8; 16384];
    while let Ok(n) = from.read(&mut buffer).await {
        if n == 0 {
            break;
        }
        if hole.load(Ordering::SeqCst) == index {
            continue;
        }
        if to.write_all(&buffer[..n]).await.is_err() {
            break;
        }
    }
}
