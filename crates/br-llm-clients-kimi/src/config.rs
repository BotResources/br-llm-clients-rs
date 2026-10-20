use std::fmt;

use br_llm_clients_common_call::{Endpoint, Secret};

use crate::kimi_thinking::KimiThinking;
use crate::route::DEFAULT_BASE;

#[derive(Clone)]
pub struct KimiConfig {
    key: Secret,
    base: Option<Endpoint>,
    thinking: Option<KimiThinking>,
}

impl KimiConfig {
    pub fn new(key: Secret) -> Self {
        Self {
            key,
            base: None,
            thinking: None,
        }
    }

    pub fn with_base(mut self, base: Endpoint) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_thinking(mut self, thinking: KimiThinking) -> Self {
        self.thinking = Some(thinking);
        self
    }

    pub(crate) fn key(&self) -> &Secret {
        &self.key
    }

    pub(crate) fn base(&self) -> Option<&Endpoint> {
        self.base.as_ref()
    }

    pub(crate) fn thinking(&self) -> Option<&KimiThinking> {
        self.thinking.as_ref()
    }

    pub(crate) fn base_text(&self) -> String {
        self.base
            .as_ref()
            .map_or_else(|| DEFAULT_BASE.to_owned(), ToString::to_string)
    }
}

impl fmt::Debug for KimiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KimiConfig")
            .field("base", &self.base_text())
            .field("thinking", &self.thinking)
            .finish_non_exhaustive()
    }
}
