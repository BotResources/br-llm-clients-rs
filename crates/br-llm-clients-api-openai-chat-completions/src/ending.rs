use br_llm_messages::ResponseEnding;

pub(crate) enum Finish {
    Error,
    Ended(Ended),
}

pub(crate) enum Ended {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Other(ResponseEnding),
}

impl Finish {
    pub(crate) fn of_reason(reason: &str) -> Option<Self> {
        let ended = match reason {
            "error" => return Some(Finish::Error),
            "stop" => Ended::Stop,
            "length" => Ended::Length,
            "tool_calls" => Ended::ToolCalls,
            "content_filter" => Ended::ContentFilter,
            other => Ended::Other(ResponseEnding::unclassified(other).ok()?),
        };
        Some(Finish::Ended(ended))
    }
}

impl Ended {
    pub(crate) fn is_length(&self) -> bool {
        matches!(self, Ended::Length)
    }

    pub(crate) fn ending(self, refusal_seen: bool) -> ResponseEnding {
        match self {
            Ended::Stop if refusal_seen => ResponseEnding::refusal(),
            Ended::Stop => ResponseEnding::finished(),
            Ended::Length => ResponseEnding::output_exhaustion(),
            Ended::ToolCalls => ResponseEnding::tool_use(),
            Ended::ContentFilter => ResponseEnding::filtering(),
            Ended::Other(ending) => ending,
        }
    }
}
