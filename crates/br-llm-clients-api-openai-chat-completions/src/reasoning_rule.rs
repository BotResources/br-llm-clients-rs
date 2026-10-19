use crate::reasoning_effort::ReasoningEffort;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReasoningRule {
    Send(ReasoningEffort),
    Omit,
    Refuse,
}
