use std::fmt;

use br_llm_clients_core::ReasoningLevel;

use crate::reasoning_effort::ReasoningEffort;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OptionsError {
    ReasoningTableDecreasing {
        level: ReasoningLevel,
        earlier: ReasoningLevel,
    },
    OffAboveNone {
        effort: ReasoningEffort,
    },
    OmitAboveOff {
        level: ReasoningLevel,
    },
    NoneAboveOff {
        level: ReasoningLevel,
    },
    BlankReasoningField,
    ReservedReasoningField {
        field: String,
    },
    BlankCacheReadField,
    ReservedExtraBodyMember {
        member: String,
    },
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionsError::ReasoningTableDecreasing { level, earlier } => write!(
                f,
                "the reasoning table sends a lower effort for {level:?} than for the lower level {earlier:?}"
            ),
            OptionsError::OffAboveNone { effort } => write!(
                f,
                "the reasoning table maps Off to the effort {effort:?}, which is above none"
            ),
            OptionsError::OmitAboveOff { level } => write!(
                f,
                "the reasoning table omits the level {level:?}, which is above Off"
            ),
            OptionsError::NoneAboveOff { level } => write!(
                f,
                "the reasoning table maps the level {level:?}, which is above Off, to none"
            ),
            OptionsError::BlankReasoningField => f.write_str("the reasoning field name is blank"),
            OptionsError::ReservedReasoningField { field } => write!(
                f,
                "the reasoning field name `{field}` is a field that the decoder already reads"
            ),
            OptionsError::BlankCacheReadField => f.write_str("the cache read field name is blank"),
            OptionsError::ReservedExtraBodyMember { member } => write!(
                f,
                "the extra body member `{member}` is written by the dialect"
            ),
        }
    }
}

impl std::error::Error for OptionsError {}
