use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::ResponseEnding;

use crate::failure_map::malformed;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Json {
    Complete,
    Incomplete,
    Malformed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Held {
    Text { nonblank: bool },
    Thinking,
    Redacted,
    Structured(Json),
    ToolCall(Json),
}

struct Content {
    texts: usize,
    structured: usize,
    tool_calls: usize,
}

impl Content {
    fn of(held: &[Held]) -> Self {
        let count = |wanted: fn(&Held) -> bool| held.iter().filter(|block| wanted(block)).count();
        Self {
            texts: count(|block| matches!(block, Held::Text { nonblank: true })),
            structured: count(|block| matches!(block, Held::Structured(_))),
            tool_calls: count(|block| matches!(block, Held::ToolCall(_))),
        }
    }

    fn is_empty(&self) -> bool {
        self.texts == 0 && self.structured == 0 && self.tool_calls == 0
    }
}

fn blocked(stop: &ResponseEnding) -> bool {
    matches!(
        stop,
        ResponseEnding::Filtering { .. } | ResponseEnding::Refusal { .. }
    )
}

fn json_of(block: &Held) -> Option<&Json> {
    match block {
        Held::Structured(json) | Held::ToolCall(json) => Some(json),
        Held::Text { .. } | Held::Thinking | Held::Redacted => None,
    }
}

fn check_json(held: &[Held], stop: &ResponseEnding) -> Result<(), ClientError> {
    let states = || held.iter().filter_map(json_of);
    if states().any(|json| *json == Json::Incomplete) {
        return Err(match stop {
            _ if blocked(stop) => ClientError::ContentBlocked,
            ResponseEnding::OutputExhaustion { .. } => ClientError::ReplyUnusable {
                ending: stop.clone(),
            },
            _ => malformed("the JSON of the reply is not complete"),
        });
    }
    if let Some(Json::Malformed(message)) = states().find(|json| matches!(json, Json::Malformed(_)))
    {
        return Err(malformed(&format!(
            "the JSON of the reply is not valid: {message}"
        )));
    }
    Ok(())
}

fn check_usable(
    content: &Content,
    stop: &ResponseEnding,
    output: &OutputMode,
) -> Result<(), ClientError> {
    let nothing_to_show = match output {
        OutputMode::Structured(_) => content.structured == 0 && content.tool_calls == 0,
        _ => content.is_empty(),
    };
    if blocked(stop) && nothing_to_show {
        return Err(ClientError::ContentBlocked);
    }
    if content.is_empty() {
        return Err(ClientError::ReplyUnusable {
            ending: stop.clone(),
        });
    }
    Ok(())
}

fn corrected(content: &Content, stop: &ResponseEnding) -> ResponseEnding {
    if content.tool_calls > 0 {
        ResponseEnding::tool_use()
    } else if matches!(stop, ResponseEnding::ToolUse { .. }) {
        ResponseEnding::finished()
    } else {
        stop.clone()
    }
}

fn check_output(
    content: &Content,
    output: &OutputMode,
    tools_declared: bool,
) -> Result<(), ClientError> {
    if !matches!(output, OutputMode::Structured(_)) {
        return Ok(());
    }
    let one_value = content.structured == 1 && content.texts == 0 && content.tool_calls == 0;
    let calls_instead = tools_declared && content.tool_calls > 0 && content.structured == 0;
    if one_value || calls_instead {
        Ok(())
    } else {
        Err(malformed(
            "the reply does not fit the structured output mode",
        ))
    }
}

pub(crate) fn judge(
    held: &[Held],
    stop: &ResponseEnding,
    output: &OutputMode,
    tools_declared: bool,
) -> Result<ResponseEnding, ClientError> {
    if matches!(stop, ResponseEnding::ProviderContinuation { .. }) {
        return Err(malformed(
            "the provider ending provider_continuation is not given by this version",
        ));
    }
    check_json(held, stop)?;
    let content = Content::of(held);
    check_usable(&content, stop, output)?;
    check_output(&content, output, tools_declared)?;
    Ok(corrected(&content, stop))
}
