#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MockBuildError {
    #[error("the usage breaks the counting rules of usage")]
    UsageBreaksCounting,
    #[error("{requested} events asked but only {available} come before the finish event")]
    EventCountOutOfRange { requested: usize, available: usize },
    #[error("gate position {position} is above the {available} events the reply can send")]
    GatePositionOutOfRange { position: usize, available: usize },
    #[error("the size of a piece of text is zero")]
    ChunkSizeZero,
    #[error("a chunking or an id renaming needs a reply that has a step")]
    StepSettingWithoutStep,
    #[error("the time to the first event is above the total duration")]
    FirstEventAfterEnd,
    #[error("a time to the first event is set on a reply that sends no event")]
    FirstEventWithoutEvent,
    #[error("a rule with a limit of zero uses can never serve a call")]
    RuleLimitZero,
}
