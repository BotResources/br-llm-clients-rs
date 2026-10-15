use std::io;

use h2::Reason;

use crate::texts::find_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    Unprocessed { written: bool, retire: bool },
    Interrupted,
    Protocol,
}

pub(crate) fn classify(error: &hyper::Error, may_have_written: bool, h2: bool) -> Cause {
    if let Some(h2_error) = find_source::<h2::Error>(error) {
        return classify_h2(h2_error);
    }
    if error.is_parse() || error.is_parse_version_h2() {
        return Cause::Protocol;
    }
    if let Some(io_error) = find_source::<io::Error>(error) {
        let tls = io_error
            .get_ref()
            .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some());
        return match io_error.kind() {
            io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput if !tls && !h2 => {
                Cause::Protocol
            }
            _ => untaken_or(may_have_written, Cause::Interrupted),
        };
    }
    if error.is_user() || error.is_canceled() || error.is_closed() {
        return untaken_or(may_have_written, Cause::Interrupted);
    }
    Cause::Interrupted
}

fn untaken_or(may_have_written: bool, otherwise: Cause) -> Cause {
    if may_have_written {
        otherwise
    } else {
        Cause::Unprocessed {
            written: false,
            retire: true,
        }
    }
}

fn classify_h2(error: &h2::Error) -> Cause {
    if error.is_remote() && error.reason() == Some(Reason::HTTP_1_1_REQUIRED) {
        return Cause::Protocol;
    }
    let refused = error.is_reset() && error.reason() == Some(Reason::REFUSED_STREAM);
    if error.is_remote() && (error.is_go_away() || refused) {
        return Cause::Unprocessed {
            written: true,
            retire: !refused,
        };
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
