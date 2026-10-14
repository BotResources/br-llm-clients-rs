use std::io;

use h2::Reason;

use crate::texts::find_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    Unprocessed { written: bool },
    Interrupted,
    Protocol,
}

pub(crate) fn classify(error: &hyper::Error, request_taken: bool) -> Cause {
    if let Some(h2_error) = find_source::<h2::Error>(error) {
        return classify_h2(h2_error);
    }
    if error.is_parse() || error.is_parse_version_h2() {
        return Cause::Protocol;
    }
    if let Some(io_error) = find_source::<io::Error>(error) {
        return match io_error.kind() {
            io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput => Cause::Protocol,
            _ => untaken_or(request_taken, Cause::Interrupted),
        };
    }
    if error.is_user() || error.is_canceled() || error.is_closed() {
        return untaken_or(request_taken, Cause::Interrupted);
    }
    Cause::Interrupted
}

fn untaken_or(request_taken: bool, otherwise: Cause) -> Cause {
    if request_taken {
        otherwise
    } else {
        Cause::Unprocessed { written: false }
    }
}

fn classify_h2(error: &h2::Error) -> Cause {
    let refused = error.is_reset() && error.reason() == Some(Reason::REFUSED_STREAM);
    if error.is_remote() && (error.is_go_away() || refused) {
        return Cause::Unprocessed { written: true };
    }
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
