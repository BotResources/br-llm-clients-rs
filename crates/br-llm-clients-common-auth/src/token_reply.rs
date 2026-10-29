use br_llm_clients_common_http::Secret;

use crate::access_token::AccessToken;

#[derive(Debug, Clone)]
pub struct TokenReply {
    pub access_token: AccessToken,
    pub refresh_token: Option<Secret>,
}
