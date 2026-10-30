use br_llm_clients_core::{ClientError, ClientRequest, RenderedHistory, RenderedRole};
use serde_json::Value;

use crate::encode_assistant::{assistant_message, plan_assistant};
use crate::encode_parts::{invalid, message};
use crate::encode_plan::{Planned, push_merged};
use crate::encode_user::{plan_user, user_messages};
use crate::options::ChatCompletionsOptions;

pub(crate) fn messages(
    request: &ClientRequest,
    history: &RenderedHistory,
    options: &ChatCompletionsOptions,
) -> Result<Vec<Value>, ClientError> {
    let plans = plan(history, options)?;
    match plans.last() {
        None => {
            return Err(invalid(
                "the history holds no message that this dialect sends",
            ));
        }
        Some(Planned::Assistant(_)) => {
            return Err(invalid(
                "the last message is an assistant message and this dialect sends no prefill",
            ));
        }
        Some(Planned::User(_)) => {}
    }
    let mut encoded = Vec::new();
    if let Some(system) = request.system() {
        let mut system_message = message("system");
        system_message.insert("content".to_owned(), Value::from(system.as_str()));
        encoded.push(Value::Object(system_message));
    }
    for planned in plans {
        match planned {
            Planned::User(user) => encoded.extend(user_messages(user)),
            Planned::Assistant(assistant) => encoded.push(assistant_message(assistant, options)),
        }
    }
    Ok(encoded)
}

fn plan(
    history: &RenderedHistory,
    options: &ChatCompletionsOptions,
) -> Result<Vec<Planned>, ClientError> {
    let mut plans = Vec::new();
    for rendered in history.messages() {
        let planned = match rendered.role() {
            RenderedRole::User => Planned::User(plan_user(rendered.parts(), options)?),
            RenderedRole::Assistant => Planned::Assistant(plan_assistant(rendered.parts())?),
        };
        push_merged(&mut plans, planned);
    }
    Ok(plans)
}
