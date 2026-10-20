use br_llm_clients_common_http::Headers;
use br_llm_messages::ModelId;

use crate::call_spec::CallSpec;

pub(crate) fn answered_model(spec: &CallSpec, headers: &Headers) -> Option<ModelId> {
    let text = spec.dialect.answered_model(headers)?;
    if spec.secrets_in_effect().appears_in(&text) {
        return None;
    }
    ModelId::new(text).ok()
}
