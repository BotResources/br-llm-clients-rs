use br_llm_clients_common_http::Headers;
use br_llm_messages::ModelName;

use crate::call_spec::CallSpec;

pub(crate) fn answered_model(spec: &CallSpec, headers: &Headers) -> Option<ModelName> {
    let text = spec.dialect.answered_model(headers)?;
    if spec.secrets_in_effect().appears_in(&text) {
        return None;
    }
    ModelName::new(text).ok()
}
