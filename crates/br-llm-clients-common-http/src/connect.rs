use std::future::poll_fn;
use std::pin::{Pin, pin};
use std::task::Poll;

use hyper::client::conn::{http1, http2};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use rustls_pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, lookup_host};
use tokio_rustls::TlsConnector;

use crate::dial::{STAGGER, first_connected};
use crate::driver::drive;
use crate::endpoint::Endpoint;
use crate::idle::IdleClock;
use crate::payload::Payload;
use crate::texts;
use crate::transport::{H1Conn, H2Sender, H2State, Signal};
use crate::tuning::Tuning;

const MAX_HEAD_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_HEADERS: usize = 1024;

pub(crate) enum ConnectFailure {
    Connect(String),
    Tls(String),
}

pub(crate) enum Opened {
    H1(H1Conn),
    H2 {
        sender: H2Sender,
        state: std::sync::Arc<H2State>,
    },
}

pub(crate) async fn open(
    tls: &TlsConnector,
    endpoint: &Endpoint,
    signal: &Signal,
    tuning: Tuning,
) -> Result<Opened, ConnectFailure> {
    let tcp = connect_tcp(endpoint).await?;
    if !endpoint.is_https() {
        return start(tcp, false, signal, tuning).await;
    }
    let name = ServerName::try_from(endpoint.dns_name().to_owned())
        .map_err(|_| ConnectFailure::Tls("the host is not a valid server name".to_owned()))?;
    let stream = tls
        .connect(name, tcp)
        .await
        .map_err(|error| ConnectFailure::Tls(texts::tls_failure(&error)))?;
    let h2 = stream.get_ref().1.alpn_protocol() == Some(b"h2".as_slice());
    start(stream, h2, signal, tuning).await
}

async fn connect_tcp(endpoint: &Endpoint) -> Result<TcpStream, ConnectFailure> {
    let addresses: Vec<_> = lookup_host((endpoint.dns_name(), endpoint.effective_port()))
        .await
        .map_err(|error| ConnectFailure::Connect(texts::dns_failure(&error)))?
        .collect();
    first_connected(addresses, STAGGER, |address| async move {
        let stream = TcpStream::connect(address).await?;
        let _ = stream.set_nodelay(true);
        Ok(stream)
    })
    .await
    .map_err(|error| ConnectFailure::Connect(texts::tcp_failure(&error)))
}

async fn start<IO>(
    io: IO,
    h2: bool,
    signal: &Signal,
    tuning: Tuning,
) -> Result<Opened, ConnectFailure>
where
    IO: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let io = TokioIo::new(io);
    let failed = |_: hyper::Error| {
        ConnectFailure::Connect("the HTTP handshake with the server failed".to_owned())
    };
    let (idle, idle_rx) = IdleClock::new(tuning.idle_limit);
    if h2 {
        let (sender, connection) = http2::Builder::new(TokioExecutor::new())
            .timer(TokioTimer::new())
            .keep_alive_interval(Some(tuning.ping_interval))
            .keep_alive_timeout(tuning.ping_timeout)
            .keep_alive_while_idle(true)
            .initial_max_send_streams(usize::MAX)
            .max_header_list_size(
                u32::try_from(MAX_HEAD_BYTES.saturating_add(1)).unwrap_or(u32::MAX),
            )
            .handshake::<_, Payload>(io)
            .await
            .map_err(failed)?;
        let state = H2State::new(idle, std::sync::Arc::clone(signal));
        state.arm_idle();
        let tracked = track_streams(connection, std::sync::Arc::clone(&state));
        tokio::spawn(drive(tracked, idle_rx));
        Ok(Opened::H2 { sender, state })
    } else {
        let (sender, connection) = http1::Builder::new()
            .max_buf_size(MAX_HEAD_BYTES)
            .max_header_size(MAX_HEAD_BYTES)
            .max_headers(MAX_RESPONSE_HEADERS)
            .handshake::<_, Payload>(io)
            .await
            .map_err(failed)?;
        let driver = tokio::spawn(drive(connection, idle_rx));
        Ok(Opened::H1(H1Conn::new(sender, idle, driver.abort_handle())))
    }
}

async fn track_streams<T>(
    connection: http2::Connection<T, Payload, TokioExecutor>,
    state: std::sync::Arc<H2State>,
) where
    T: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let mut connection = pin!(connection);
    let _ = poll_fn(|cx| {
        let polled = Pin::as_mut(&mut connection).poll(cx);
        state.set_max_streams(connection.as_ref().current_max_send_streams());
        match polled {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => Poll::Pending,
        }
    })
    .await;
}
