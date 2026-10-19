#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReasoningSummary {
    None,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReasoningParameter {
    #[default]
    Sent,
    Omitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Options {
    pub reasoning_summary: ReasoningSummary,
    pub reasoning_parameter: ReasoningParameter,
}
