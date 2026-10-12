use br_llm_messages::Usage;

use crate::build_error::MockBuildError;

pub(crate) fn check(usage: &Usage) -> Result<(), MockBuildError> {
    let cached = usage
        .cache_read_tokens
        .unwrap_or(0)
        .saturating_add(usage.cache_write_tokens.unwrap_or(0));
    let thinking = usage.thinking_tokens.unwrap_or(0);
    if cached > usage.input_tokens || thinking > usage.output_tokens {
        return Err(MockBuildError::UsageBreaksCounting);
    }
    Ok(())
}
