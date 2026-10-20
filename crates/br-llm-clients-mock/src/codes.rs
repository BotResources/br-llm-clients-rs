use br_llm_clients_core::ClientError;

pub const CODE_SCRIPT_EXHAUSTED: &str = "mock.script_exhausted";
pub const CODE_INVALID_REPLY: &str = "mock.invalid_reply";

fn rejected(code: &str, body: String) -> ClientError {
    ClientError::ProviderRejected {
        status: None,
        code: Some(code.to_owned()),
        body,
    }
}

pub(crate) fn script_exhausted() -> ClientError {
    rejected(
        CODE_SCRIPT_EXHAUSTED,
        "no rule matched, the script has no entry left and the mock has no fallback".to_owned(),
    )
}

pub(crate) fn invalid_reply(origin: &str, cause: &impl std::fmt::Display) -> ClientError {
    rejected(
        CODE_INVALID_REPLY,
        format!("{origin} returned an invalid reply: {cause}"),
    )
}
