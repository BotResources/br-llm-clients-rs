use std::net::SocketAddr;

use hyper::client::conn::{http1, http2};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls_pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, lookup_host};
use tokio_rustls::TlsConnector;

use crate::endpoint::Endpoint;
use crate::payload::Payload;
use crate::redact::Redactor;
use crate::transport::{Conn, H1Conn};

const MAX_HEAD_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_HEADERS: usize = 1024;

pub(crate) enum ConnectFailure {
    Connect(String),
    Tls(String),
    Handshake(String),
}

pub(crate) async fn open(
    tls: &TlsConnector,
    endpoint: &Endpoint,
    redactor: &Redactor,
) -> Result<Conn, ConnectFailure> {
    let tcp = connect_tcp(endpoint, redactor).await?;
    if !endpoint.is_https() {
        return start(tcp, false, redactor).await;
    }
    let name = ServerName::try_from(endpoint.dns_name().to_owned())
        .map_err(|_| ConnectFailure::Tls("the host is not a valid server name".to_owned()))?;
    let stream = tls
        .connect(name, tcp)
        .await
        .map_err(|error| ConnectFailure::Tls(redactor.describe(&error)))?;
    let h2 = stream.get_ref().1.alpn_protocol() == Some(b"h2".as_slice());
    start(stream, h2, redactor).await
}

async fn connect_tcp(
    endpoint: &Endpoint,
    redactor: &Redactor,
) -> Result<TcpStream, ConnectFailure> {
    let fail = |error: std::io::Error| ConnectFailure::Connect(redactor.describe(&error));
    let addresses: Vec<SocketAddr> = lookup_host((endpoint.dns_name(), endpoint.effective_port()))
        .await
        .map_err(fail)?
        .collect();
    let mut last = None;
    for address in addresses {
        match TcpStream::connect(address).await {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
            Err(error) => last = Some(error),
        }
    }
    Err(match last {
        Some(error) => fail(error),
        None => ConnectFailure::Connect("name resolution gave no address".to_owned()),
    })
}

async fn start<IO>(io: IO, h2: bool, redactor: &Redactor) -> Result<Conn, ConnectFailure>
where
    IO: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let io = TokioIo::new(io);
    let handshake_failed =
        |error: hyper::Error| ConnectFailure::Handshake(redactor.describe(&error));
    if h2 {
        let (sender, connection) = http2::Builder::new(TokioExecutor::new())
            .max_header_list_size(u32::try_from(MAX_HEAD_BYTES).unwrap_or(u32::MAX))
            .handshake::<_, Payload>(io)
            .await
            .map_err(handshake_failed)?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Conn::H2(sender))
    } else {
        let (sender, connection) = http1::Builder::new()
            .max_buf_size(MAX_HEAD_BYTES)
            .max_header_size(MAX_HEAD_BYTES)
            .max_headers(MAX_RESPONSE_HEADERS)
            .handshake::<_, Payload>(io)
            .await
            .map_err(handshake_failed)?;
        let driver = tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Conn::H1(H1Conn::new(sender, driver.abort_handle())))
    }
}
