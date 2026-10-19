#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl ReasoningEffort {
    pub(crate) fn rank(self) -> u8 {
        match self {
            ReasoningEffort::None => 0,
            ReasoningEffort::Minimal => 1,
            ReasoningEffort::Low => 2,
            ReasoningEffort::Medium => 3,
            ReasoningEffort::High => 4,
            ReasoningEffort::Xhigh => 5,
            ReasoningEffort::Max => 6,
        }
    }

    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            ReasoningEffort::None => "none",
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::Xhigh => "xhigh",
            ReasoningEffort::Max => "max",
        }
    }
}
