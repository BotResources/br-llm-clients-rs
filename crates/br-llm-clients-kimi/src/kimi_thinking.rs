#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum KimiThinking {
    EffortScale,
    ThinkingSwitch,
    AlwaysThinking,
}

impl KimiThinking {
    pub(crate) fn of_model(model: &str) -> Option<Self> {
        match model {
            "kimi-k3" => Some(Self::EffortScale),
            "kimi-k2.6" => Some(Self::ThinkingSwitch),
            "kimi-k2.7-code" | "kimi-k2.7-code-highspeed" => Some(Self::AlwaysThinking),
            _ => None,
        }
    }
}
