use br_llm_clients_core::ReasoningLevel;

use crate::options_error::OptionsError;
use crate::reasoning_effort::ReasoningEffort;
use crate::reasoning_rule::ReasoningRule;

#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningTable {
    off: ReasoningRule,
    low: ReasoningRule,
    medium: ReasoningRule,
    high: ReasoningRule,
    extra: ReasoningRule,
    max: ReasoningRule,
}

impl ReasoningTable {
    pub fn new(
        off: ReasoningRule,
        low: ReasoningRule,
        medium: ReasoningRule,
        high: ReasoningRule,
        extra: ReasoningRule,
        max: ReasoningRule,
    ) -> Self {
        Self {
            off,
            low,
            medium,
            high,
            extra,
            max,
        }
    }

    pub fn openai_effort() -> Self {
        Self::new(
            ReasoningRule::Send(ReasoningEffort::None),
            ReasoningRule::Send(ReasoningEffort::Low),
            ReasoningRule::Send(ReasoningEffort::Medium),
            ReasoningRule::Send(ReasoningEffort::High),
            ReasoningRule::Send(ReasoningEffort::Xhigh),
            ReasoningRule::Send(ReasoningEffort::Max),
        )
    }

    pub fn openai_effort_without_none() -> Self {
        Self::new(
            ReasoningRule::Refuse,
            ReasoningRule::Send(ReasoningEffort::Low),
            ReasoningRule::Send(ReasoningEffort::Medium),
            ReasoningRule::Send(ReasoningEffort::High),
            ReasoningRule::Send(ReasoningEffort::High),
            ReasoningRule::Send(ReasoningEffort::High),
        )
    }

    pub(crate) fn rule_for(&self, level: ReasoningLevel) -> Option<ReasoningRule> {
        match level {
            ReasoningLevel::Off => Some(self.off),
            ReasoningLevel::Low => Some(self.low),
            ReasoningLevel::Medium => Some(self.medium),
            ReasoningLevel::High => Some(self.high),
            ReasoningLevel::Extra => Some(self.extra),
            ReasoningLevel::Max => Some(self.max),
            _ => None,
        }
    }

    pub(crate) fn rules(&self) -> [(ReasoningLevel, ReasoningRule); 6] {
        [
            (ReasoningLevel::Off, self.off),
            (ReasoningLevel::Low, self.low),
            (ReasoningLevel::Medium, self.medium),
            (ReasoningLevel::High, self.high),
            (ReasoningLevel::Extra, self.extra),
            (ReasoningLevel::Max, self.max),
        ]
    }

    pub(crate) fn check(&self, omit_above_off: bool) -> Result<(), OptionsError> {
        let mut highest: Option<(ReasoningLevel, ReasoningEffort)> = None;
        for (level, rule) in self.rules() {
            check_rule(level, rule, omit_above_off)?;
            if let ReasoningRule::Send(effort) = rule {
                if let Some((earlier, top)) = highest
                    && effort.rank() < top.rank()
                {
                    return Err(OptionsError::ReasoningTableDecreasing { level, earlier });
                }
                if highest.is_none_or(|(_, top)| effort.rank() > top.rank()) {
                    highest = Some((level, effort));
                }
            }
        }
        Ok(())
    }
}

fn check_rule(
    level: ReasoningLevel,
    rule: ReasoningRule,
    omit_above_off: bool,
) -> Result<(), OptionsError> {
    let is_off = level == ReasoningLevel::Off;
    match rule {
        ReasoningRule::Send(effort) if is_off && effort != ReasoningEffort::None => {
            Err(OptionsError::OffAboveNone { effort })
        }
        ReasoningRule::Send(ReasoningEffort::None) if !is_off => {
            Err(OptionsError::NoneAboveOff { level })
        }
        ReasoningRule::Omit if !is_off && !omit_above_off => {
            Err(OptionsError::OmitAboveOff { level })
        }
        ReasoningRule::Send(_) | ReasoningRule::Omit | ReasoningRule::Refuse => Ok(()),
    }
}
