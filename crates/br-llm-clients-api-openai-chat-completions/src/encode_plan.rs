use serde_json::Value;

pub(crate) enum Planned {
    User(UserPlan),
    Assistant(AssistantPlan),
}

#[derive(Default)]
pub(crate) struct UserPlan {
    pub(crate) tools: Vec<Value>,
    pub(crate) parts: Vec<UserPart>,
}

pub(crate) enum UserPart {
    Text(String),
    Image(Value),
}

#[derive(Default)]
pub(crate) struct AssistantPlan {
    pub(crate) texts: Vec<String>,
    pub(crate) calls: Vec<Value>,
    pub(crate) thinking: String,
}

impl Planned {
    fn is_empty(&self) -> bool {
        match self {
            Planned::User(user) => user.tools.is_empty() && user.parts.is_empty(),
            Planned::Assistant(assistant) => {
                assistant.texts.is_empty() && assistant.calls.is_empty()
            }
        }
    }
}

pub(crate) fn push_merged(plans: &mut Vec<Planned>, next: Planned) {
    if next.is_empty() {
        return;
    }
    match (plans.last_mut(), next) {
        (Some(Planned::User(last)), Planned::User(next)) => {
            last.tools.extend(next.tools);
            last.parts.extend(next.parts);
        }
        (Some(Planned::Assistant(last)), Planned::Assistant(next)) => {
            last.texts.extend(next.texts);
            last.calls.extend(next.calls);
            last.thinking.push_str(&next.thinking);
        }
        (_, next) => plans.push(next),
    }
}
