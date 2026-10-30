use std::sync::Arc;

use br_llm_clients_common_http::{ApiKeyAuthorizer, Authorizer, BearerAuthorizer, Secret};

use crate::build_error::AnthropicBuildError;
use crate::config::KeyHeader;

pub(crate) fn check_key(key: &Secret) -> Result<(), AnthropicBuildError> {
    if is_visible_text(key.expose()) {
        return Ok(());
    }
    Err(AnthropicBuildError::credential(
        "the key must hold only visible characters",
    ))
}

pub(crate) fn check_workspace_id(workspace_id: Option<&str>) -> Result<(), AnthropicBuildError> {
    match workspace_id {
        Some(id) if !is_visible_text(id) => Err(AnthropicBuildError::workspace_id(
            "the workspace id must be a non-empty text of visible characters",
        )),
        Some(_) | None => Ok(()),
    }
}

pub(crate) fn authorizer(
    header: KeyHeader,
    key: &Secret,
) -> Result<Arc<dyn Authorizer>, AnthropicBuildError> {
    match header {
        KeyHeader::ApiKey => ApiKeyAuthorizer::new(header.name(), key.clone())
            .map(|authorizer| Arc::new(authorizer) as Arc<dyn Authorizer>)
            .map_err(|_| {
                AnthropicBuildError::credential(
                    "the HTTP library refused the fixed key header name x-api-key",
                )
            }),
        KeyHeader::Bearer => Ok(Arc::new(BearerAuthorizer::new(key))),
    }
}

fn is_visible_text(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| (0x21..=0x7E).contains(&byte))
}
