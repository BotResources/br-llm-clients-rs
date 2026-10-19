#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::wildcard_enum_match_arm
)]

use crate::cache::CacheRequirement;
use crate::capabilities::Capabilities;
use crate::capabilities_check_images::check_images;
use crate::client_error::ClientError;
use crate::client_request::ClientRequest;
use crate::client_settings::ClientSettings;
use crate::output_mode::OutputMode;

impl Capabilities {
    pub fn check(
        &self,
        settings: &ClientSettings,
        request: &ClientRequest,
    ) -> Result<(), ClientError> {
        self.check_reasoning(settings)?;
        self.check_cache(settings)?;
        self.check_output_tokens(settings)?;
        self.check_tools(request)?;
        self.check_structured_output(request)?;
        self.check_structured_output_with_tools(request)?;
        check_images(self, request)
    }

    fn check_reasoning(&self, settings: &ClientSettings) -> Result<(), ClientError> {
        let level = settings.reasoning();
        if self.reasoning_levels().accepts(&level) {
            return Ok(());
        }
        Err(ClientError::ReasoningNotSupported {
            level,
            reason: format!(
                "the declared capabilities of model {} do not accept this level",
                settings.model()
            ),
        })
    }

    fn check_cache(&self, settings: &ClientSettings) -> Result<(), ClientError> {
        match settings.cache() {
            CacheRequirement::Required if !self.caching() => Err(ClientError::CacheNotSupported),
            CacheRequirement::Required | CacheRequirement::NotRequired => Ok(()),
        }
    }

    fn check_output_tokens(&self, settings: &ClientSettings) -> Result<(), ClientError> {
        let limit = self.max_output_tokens();
        if settings.max_output_tokens() > limit {
            return Err(ClientError::MaxOutputTokensTooLarge {
                limit: Some(u64::from(limit.get())),
            });
        }
        Ok(())
    }

    fn check_tools(&self, request: &ClientRequest) -> Result<(), ClientError> {
        if !request.tools().is_empty() && !self.tools() {
            return Err(ClientError::ToolsNotSupported);
        }
        Ok(())
    }

    fn check_structured_output(&self, request: &ClientRequest) -> Result<(), ClientError> {
        match request.output() {
            OutputMode::Structured(_) if !self.structured_output() => {
                Err(ClientError::StructuredOutputNotSupported)
            }
            OutputMode::Structured(_) | OutputMode::Plain => Ok(()),
        }
    }

    fn check_structured_output_with_tools(
        &self,
        request: &ClientRequest,
    ) -> Result<(), ClientError> {
        match request.output() {
            OutputMode::Structured(_)
                if !request.tools().is_empty() && !self.accepts_structured_output_with_tools() =>
            {
                Err(ClientError::StructuredOutputWithToolsNotSupported)
            }
            OutputMode::Structured(_) | OutputMode::Plain => Ok(()),
        }
    }
}
