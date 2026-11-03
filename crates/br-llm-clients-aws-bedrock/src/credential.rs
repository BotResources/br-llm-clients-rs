use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::Secret;

use crate::aws_credentials::CredentialSource;

#[derive(Clone)]
#[non_exhaustive]
pub enum BedrockCredential {
    ApiKey(Secret),
    Sigv4(Arc<dyn CredentialSource>),
}

impl fmt::Debug for BedrockCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKey(_) => f.write_str("ApiKey"),
            Self::Sigv4(_) => f.write_str("Sigv4"),
        }
    }
}
