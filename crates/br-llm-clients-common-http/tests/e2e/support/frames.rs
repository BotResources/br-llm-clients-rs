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

pub fn settings_frame_padded(filler: usize, limit: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    for _ in 0..filler {
        payload.extend_from_slice(&[0x77, 0x77, 0, 0, 0, 9]);
    }
    payload.extend_from_slice(&MAX_CONCURRENT_STREAMS.to_be_bytes());
    payload.extend_from_slice(&limit.to_be_bytes());
    frame(SETTINGS, 0, 0, &payload)
}

pub fn goaway_frame(last_stream: u32, code: u32, debug: &[u8]) -> Vec<u8> {
    let mut payload = last_stream.to_be_bytes().to_vec();
    payload.extend_from_slice(&code.to_be_bytes());
    payload.extend_from_slice(debug);
    frame(GOAWAY, 0, 0, &payload)
}

pub const STATUS_204: u8 = 0x89;
pub const STATUS_304: u8 = 0x8b;

pub fn response_head(stream: u32, content_length: Option<&str>, end_stream: bool) -> Vec<u8> {
    response_head_with_status(stream, STATUS_200, content_length, end_stream)
}

pub fn trailers(stream: u32) -> Vec<u8> {
    let block = [0x40, 0x03, b'x', b'-', b't', 0x01, b'1'];
    frame(HEADERS, END_STREAM | END_HEADERS, stream, &block)
}

pub fn response_head_with_status(
    stream: u32,
    status: u8,
    content_length: Option<&str>,
    end_stream: bool,
) -> Vec<u8> {
    let mut block = vec![status];
    if let Some(length) = content_length {
        block.extend_from_slice(&[0x0f, 0x0d]);
        block.push(u8::try_from(length.len()).expect("length"));
        block.extend_from_slice(length.as_bytes());
    }
    let flags = END_HEADERS | if end_stream { END_STREAM } else { 0 };
    frame(HEADERS, flags, stream, &block)
}

pub fn reset_frame(stream: u32, code: u32) -> Vec<u8> {
    frame(3, 0, stream, &code.to_be_bytes())
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
