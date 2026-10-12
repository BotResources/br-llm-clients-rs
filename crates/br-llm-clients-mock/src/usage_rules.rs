use br_llm_messages::Usage;

use crate::build_error::MockBuildError;

fn cache_tokens_exceed_input(usage: &Usage) -> bool {
    let read = usage.cache_read_tokens.unwrap_or(0);
    let write = usage.cache_write_tokens.unwrap_or(0);
    read.checked_add(write)
        .is_none_or(|cached| cached > usage.input_tokens)
}

fn thinking_tokens_exceed_output(usage: &Usage) -> bool {
    usage.thinking_tokens.unwrap_or(0) > usage.output_tokens
}

pub(crate) fn check(usage: &Usage) -> Result<(), MockBuildError> {
    if cache_tokens_exceed_input(usage) || thinking_tokens_exceed_output(usage) {
        return Err(MockBuildError::UsageBreaksCounting);
    }
    Ok(())
}
