use br_llm_messages::Usage;

use crate::build_error::MockBuildError;

fn cache_count_without_input(usage: &Usage) -> bool {
    let cached =
        usage.cache_read_input_tokens().is_some() || usage.cache_creation_input_tokens().is_some();
    cached && usage.input_tokens().is_none()
}

fn thinking_count_without_output(usage: &Usage) -> bool {
    usage.thinking_tokens().is_some() && usage.output_tokens().is_none()
}

fn cache_tokens_exceed_input(usage: &Usage) -> bool {
    let read = usage.cache_read_input_tokens().unwrap_or(0);
    let creation = usage.cache_creation_input_tokens().unwrap_or(0);
    read.checked_add(creation)
        .is_none_or(|cached| cached > usage.input_tokens().unwrap_or(0))
}

fn thinking_tokens_exceed_output(usage: &Usage) -> bool {
    usage.thinking_tokens().unwrap_or(0) > usage.output_tokens().unwrap_or(0)
}

pub(crate) fn check(usage: &Usage) -> Result<(), MockBuildError> {
    if cache_count_without_input(usage)
        || thinking_count_without_output(usage)
        || cache_tokens_exceed_input(usage)
        || thinking_tokens_exceed_output(usage)
    {
        return Err(MockBuildError::UsageBreaksCounting);
    }
    Ok(())
}
