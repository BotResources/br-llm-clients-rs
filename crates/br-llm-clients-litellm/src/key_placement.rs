#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum KeyPlacement {
    #[default]
    AuthorizationBearer,
    LiteLlmKeyHeader,
    ConfiguredKeyHeader(String),
}

impl KeyPlacement {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            KeyPlacement::AuthorizationBearer => "AuthorizationBearer",
            KeyPlacement::LiteLlmKeyHeader => "LiteLlmKeyHeader",
            KeyPlacement::ConfiguredKeyHeader(_) => "ConfiguredKeyHeader",
        }
    }
}
