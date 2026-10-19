use crate::reasoning_effort::ReasoningEffort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningRule {
    Send(ReasoningEffort),
    Omit,
    Refuse,
}
