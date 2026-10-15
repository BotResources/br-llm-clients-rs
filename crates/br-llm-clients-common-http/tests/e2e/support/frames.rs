use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const DATA: u8 = 0;
pub const HEADERS: u8 = 1;
pub const SETTINGS: u8 = 4;
pub const PING: u8 = 6;
pub const GOAWAY: u8 = 7;
pub const END_STREAM: u8 = 1;
pub const END_HEADERS: u8 = 4;
pub const ACK: u8 = 1;
pub const MAX_CONCURRENT_STREAMS: u16 = 3;
pub const STATUS_200: u8 = 0x88;

pub struct Frame {
    pub kind: u8,
    pub flags: u8,
    pub stream: u32,
    pub payload: Vec<u8>,
}

pub fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).expect("length");
    let mut out = vec![
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
        kind,
        flags,
    ];
    out.extend_from_slice(&stream.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

pub async fn read_frame<S: AsyncReadExt + Unpin>(stream: &mut S) -> Option<Frame> {
    let mut head = [0u8; 9];
    stream.read_exact(&mut head).await.ok()?;
    let length = (usize::from(head[0]) << 16) | (usize::from(head[1]) << 8) | usize::from(head[2]);
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload).await.ok()?;
    Some(Frame {
        kind: head[3],
        flags: head[4],
        stream: u32::from_be_bytes([head[5] & 0x7f, head[6], head[7], head[8]]),
        payload,
    })
}

pub fn settings_frame(max_concurrent_streams: Option<u32>) -> Vec<u8> {
    let mut payload = Vec::new();
    if let Some(limit) = max_concurrent_streams {
        payload.extend_from_slice(&MAX_CONCURRENT_STREAMS.to_be_bytes());
        payload.extend_from_slice(&limit.to_be_bytes());
    }
    frame(SETTINGS, 0, 0, &payload)
}

pub fn goaway_frame(last_stream: u32, code: u32, debug: &[u8]) -> Vec<u8> {
    let mut payload = last_stream.to_be_bytes().to_vec();
    payload.extend_from_slice(&code.to_be_bytes());
    payload.extend_from_slice(debug);
    frame(GOAWAY, 0, 0, &payload)
}

pub fn empty_response(stream: u32) -> Vec<u8> {
    frame(HEADERS, END_STREAM | END_HEADERS, stream, &[STATUS_200])
}

pub async fn read_preface<S: AsyncReadExt + Unpin>(stream: &mut S) -> bool {
    let mut preface = [0u8; 24];
    stream.read_exact(&mut preface).await.is_ok()
}

pub async fn write_all<S: AsyncWriteExt + Unpin>(stream: &mut S, bytes: &[u8]) {
    let _ = stream.write_all(bytes).await;
    let _ = stream.flush().await;
}
