use br_llm_clients_common_http::Headers;
use br_llm_messages::ModelName;

use crate::call_spec::CallSpec;
use crate::secrets_cache::SecretsCache;

pub(crate) fn answered_model(
    spec: &CallSpec,
    secrets: &SecretsCache,
    headers: &Headers,
) -> Option<ModelName> {
    let text = spec.dialect.answered_model(headers)?;
    if secrets.in_effect(spec).appears_in(&text) {
        return None;
    }
    ModelName::new(text).ok()
}
