#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MockBuildError {
    #[error("the usage breaks the counting rules of usage")]
    UsageBreaksCounting,
    #[error("{requested} deltas asked but only {available} come before the end of the response")]
    DeltaCountOutOfRange { requested: usize, available: usize },
    #[error("gate position {position} is above the {available} deltas the reply can send")]
    GatePositionOutOfRange { position: usize, available: usize },
    #[error("the size of a piece of text is zero")]
    ChunkSizeZero,
    #[error("the response sketch is invalid: {reason}")]
    SketchInvalid { reason: String },
    #[error("a chunking needs a reply that has a response")]
    ChunkingWithoutResponse,
    #[error("the time to the first delta is above the total duration")]
    FirstEventAfterEnd,
    #[error("a time to the first delta is set on a reply that sends no delta")]
    FirstEventWithoutEvent,
    #[error("a rule with a limit of zero uses can never serve a call")]
    RuleLimitZero,
    #[error("the deltas of the response cannot be derived: {reason}")]
    DerivationFailed { reason: String },
}
