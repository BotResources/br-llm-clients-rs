#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum MaxTokensField {
    MaxCompletionTokens,
    MaxTokens,
}

impl MaxTokensField {
    pub(crate) fn member(self) -> &'static str {
        match self {
            MaxTokensField::MaxCompletionTokens => "max_completion_tokens",
            MaxTokensField::MaxTokens => "max_tokens",
        }
    }
}
