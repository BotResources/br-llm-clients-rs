use br_llm_messages::ToolName;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    #[error("a request needs at least one message")]
    NoMessage,
    #[error("two tools share the name {name}")]
    DuplicateToolName { name: ToolName },
    #[error("two parameters of one list share the name {parameter:?}")]
    DuplicateParameterName { parameter: String },
    #[error("a parameter name is blank")]
    BlankParameterName,
    #[error("the enum of parameter {parameter:?} has no value")]
    EmptyEnum { parameter: String },
    #[error("the enum of parameter {parameter:?} has a blank value")]
    BlankEnumValue { parameter: String },
    #[error("the enum of parameter {parameter:?} lists {value:?} twice")]
    DuplicateEnumValue { parameter: String, value: String },
    #[error("the name of a structured shape is blank")]
    BlankShapeName,
    #[error("the number of output tokens is zero")]
    ZeroMaxOutputTokens,
    #[error("parameter {parameter:?} is nested deeper than the limit")]
    NestingTooDeep { parameter: String },
    #[error("a relay must be the last message and must not be the only one")]
    MisplacedRelay,
    #[error("the usage of the report differs from the usage of the step")]
    UsageMismatch,
}
