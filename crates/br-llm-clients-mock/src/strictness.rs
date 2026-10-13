use br_llm_clients_core::{
    CacheRequirement, ClientError, ClientRequest, ClientSettings, OutputMode,
};
use br_llm_messages::{WireMessage, WireUserBlock};

use crate::capabilities::Capabilities;
use crate::codes::unknown_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheClass {
    Required,
    NotRequired,
    Unknown,
}

pub(crate) fn classify_cache(requirement: CacheRequirement) -> CacheClass {
    if matches!(requirement, CacheRequirement::Required) {
        CacheClass::Required
    } else if matches!(requirement, CacheRequirement::NotRequired) {
        CacheClass::NotRequired
    } else {
        CacheClass::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeClass {
    Plain,
    Structured,
    Unknown,
}

pub(crate) fn classify_mode(mode: &OutputMode) -> ModeClass {
    if matches!(mode, OutputMode::Plain) {
        ModeClass::Plain
    } else if matches!(mode, OutputMode::Structured(_)) {
        ModeClass::Structured
    } else {
        ModeClass::Unknown
    }
}

pub(crate) fn check(
    capabilities: &Capabilities,
    settings: &ClientSettings,
    request: &ClientRequest,
) -> Result<(), ClientError> {
    check_settings(capabilities, settings, classify_cache(settings.cache()))?;
    check_request(capabilities, request, classify_mode(request.output()))
}

pub(crate) fn check_settings(
    capabilities: &Capabilities,
    settings: &ClientSettings,
    cache: CacheClass,
) -> Result<(), ClientError> {
    let level = settings.reasoning();
    if !capabilities.reasoning_levels().accepts(&level) {
        return Err(ClientError::ReasoningNotSupported {
            level,
            reason: "the capabilities of the mock do not accept this level".to_owned(),
        });
    }
    match cache {
        CacheClass::Required if !capabilities.caching() => {
            return Err(ClientError::CacheNotSupported);
        }
        CacheClass::Unknown => return Err(unknown_value("CacheRequirement")),
        CacheClass::Required | CacheClass::NotRequired => {}
    }
    let limit = capabilities.max_output_tokens();
    if settings.max_output_tokens() > limit {
        return Err(ClientError::MaxOutputTokensTooLarge {
            limit: Some(u64::from(limit.get())),
        });
    }
    Ok(())
}

pub(crate) fn check_request(
    capabilities: &Capabilities,
    request: &ClientRequest,
    mode: ModeClass,
) -> Result<(), ClientError> {
    request.check_messages()?;
    let has_tools = !request.tools().is_empty();
    if has_tools && !capabilities.tools() {
        return Err(ClientError::ToolsNotSupported);
    }
    match mode {
        ModeClass::Structured => {
            if !capabilities.structured_output() {
                return Err(ClientError::StructuredOutputNotSupported);
            }
            if has_tools && !capabilities.accepts_structured_output_with_tools() {
                return Err(ClientError::StructuredOutputWithToolsNotSupported);
            }
        }
        ModeClass::Unknown => return Err(unknown_value("OutputMode")),
        ModeClass::Plain => {}
    }
    check_images(capabilities, request.messages())
}

fn check_images(capabilities: &Capabilities, messages: &[WireMessage]) -> Result<(), ClientError> {
    for message in messages {
        let WireMessage::User { content } = message else {
            continue;
        };
        for block in content {
            let WireUserBlock::Image(image) = block else {
                continue;
            };
            if !capabilities.images() {
                return Err(ClientError::ImageNotSupported);
            }
            if !capabilities.image_media_types().accepts(&image.mime) {
                return Err(ClientError::ImageMediaTypeNotSupported {
                    media_type: image.mime,
                });
            }
        }
    }
    Ok(())
}
