mod calls;
mod content;
mod escape;
mod frames;
mod incoming;
mod merge;
mod notices;
mod refusal;
mod response;
mod results;
mod turns;
mod types;

#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod frames_tests;
#[cfg(test)]
mod free_order_tests;
#[cfg(test)]
mod merge_tests;
#[cfg(test)]
mod notices_runtime_tests;
#[cfg(test)]
mod notices_tests;
#[cfg(test)]
mod outstanding_tests;
#[cfg(test)]
mod placement_gap_tests;
#[cfg(test)]
mod placement_tests;
#[cfg(test)]
mod replay_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod wire_id_tests;

pub use types::{RenderedHistory, RenderedMessage, RenderedPart, RenderedResultPart, RenderedRole};

use crate::client_error::ClientError;
use crate::client_request::ClientRequest;
use refusal::invalid;

pub fn render_history(request: &ClientRequest) -> Result<RenderedHistory, ClientError> {
    let calls = calls::CallTable::of(request.history());
    let messages = turns::messages(request.history(), request.reply_turn(), &calls)?;
    if messages.is_empty() {
        return Err(invalid("the history renders no message"));
    }
    Ok(RenderedHistory::new(messages))
}
