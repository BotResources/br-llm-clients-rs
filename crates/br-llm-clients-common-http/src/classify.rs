use std::error::Error;
use std::io;

use h2::Reason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    Interrupted,
    Protocol,
}

pub(crate) fn classify(error: &hyper::Error) -> Cause {
    if error.is_parse() || error.is_parse_version_h2() || error.is_user() {
        return Cause::Protocol;
    }
    let mut current: Option<&(dyn Error + 'static)> = error.source();
    while let Some(layer) = current {
        if let Some(h2_error) = layer.downcast_ref::<h2::Error>() {
            return classify_h2(h2_error);
        }
        if let Some(io_error) = layer.downcast_ref::<io::Error>() {
            return classify_io(io_error);
        }
        current = layer.source();
    }
    Cause::Interrupted
}

fn classify_h2(error: &h2::Error) -> Cause {
    let violation = matches!(
        error.reason(),
        Some(
            Reason::PROTOCOL_ERROR
                | Reason::FRAME_SIZE_ERROR
                | Reason::COMPRESSION_ERROR
                | Reason::FLOW_CONTROL_ERROR
                | Reason::ENHANCE_YOUR_CALM
        )
    );
    if error.is_library() && violation {
        Cause::Protocol
    } else {
        Cause::Interrupted
    }
}

fn classify_io(error: &io::Error) -> Cause {
    match error.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput => Cause::Protocol,
        _ => Cause::Interrupted,
    }
}
