use std::fmt;
use std::sync::Arc;

use br_llm_clients_api_openai_chat_completions::ReasoningTable;
use br_llm_clients_common_http::{Endpoint, Secret};
use br_llm_clients_core::{Capabilities, IdSource};
use serde_json::{Map, Value};

#[derive(Clone)]
pub struct MlxVlmConfig {
    pub(crate) base: Endpoint,
    pub(crate) reasoning_table: ReasoningTable,
    pub(crate) capabilities: Capabilities,
    pub(crate) key: Option<Secret>,
    pub(crate) extra_body: Map<String, Value>,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) reasoning_carry_back: bool,
    pub(crate) id_source: Option<Arc<dyn IdSource>>,
}

impl MlxVlmConfig {
    pub fn new(
        base: Endpoint,
        reasoning_table: ReasoningTable,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            base,
            reasoning_table,
            capabilities,
            key: None,
            extra_body: Map::new(),
            extra_headers: Vec::new(),
            reasoning_carry_back: true,
            id_source: None,
        }
    }

    pub fn with_key(mut self, key: Secret) -> Self {
        self.key = Some(key);
        self
    }

    pub fn with_extra_body(mut self, extra_body: Map<String, Value>) -> Self {
        self.extra_body = extra_body;
        self
    }

    pub fn with_extra_headers(mut self, extra_headers: Vec<(String, String)>) -> Self {
        self.extra_headers = extra_headers;
        self
    }

    pub fn with_reasoning_carry_back(mut self, reasoning_carry_back: bool) -> Self {
        self.reasoning_carry_back = reasoning_carry_back;
        self
    }

    pub fn with_id_source(mut self, id_source: Arc<dyn IdSource>) -> Self {
        self.id_source = Some(id_source);
        self
    }
}

impl fmt::Debug for MlxVlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = if self.key.is_some() { "set" } else { "none" };
        let id_source = if self.id_source.is_some() {
            "set"
        } else {
            "default"
        };
        f.debug_struct("MlxVlmConfig")
            .field("base", &format_args!("{}", self.base))
            .field("key", &key)
            .field("extra_headers", &self.extra_headers.len())
            .field("reasoning_carry_back", &self.reasoning_carry_back)
            .field("id_source", &id_source)
            .finish_non_exhaustive()
    }
}
