use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use br_llm_clients_common_call::Secret;
use br_llm_clients_common_http::AuthorizerError;
use ring::rand::SystemRandom;
use ring::signature::RSA_PKCS1_SHA256;
use serde_json::{Value, json};

use crate::service_account_key::ServiceAccountKey;

const SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
const LIFETIME_SECONDS: u64 = 3600;

fn unsignable(reason: &str) -> AuthorizerError {
    AuthorizerError::unavailable(format!("the token assertion cannot be signed: {reason}"))
}

pub(crate) fn sign(key: &ServiceAccountKey) -> Result<Secret, AuthorizerError> {
    let issued = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| unsignable("the system clock is before 1970"))?
        .as_secs();
    let expires = issued
        .checked_add(LIFETIME_SECONDS)
        .ok_or_else(|| unsignable("the system clock is too far in the future"))?;
    let mut header = json!({"alg": "RS256", "typ": "JWT"});
    if let (Some(id), Some(members)) = (key.private_key_id(), header.as_object_mut()) {
        members.insert("kid".to_owned(), json!(id));
    }
    let claims = json!({
        "iss": key.client_email(),
        "scope": SCOPE,
        "aud": key.token_uri(),
        "iat": issued,
        "exp": expires,
    });
    let mut unsigned = encoded(&header)?;
    unsigned.push('.');
    unsigned.push_str(&encoded(&claims)?);
    let key_pair = key.key_pair();
    let mut signature = vec![0u8; key_pair.public().modulus_len()];
    key_pair
        .sign(
            &RSA_PKCS1_SHA256,
            &SystemRandom::new(),
            unsigned.as_bytes(),
            &mut signature,
        )
        .map_err(|_| unsignable("the signer refused the private key"))?;
    unsigned.push('.');
    unsigned.push_str(&URL_SAFE_NO_PAD.encode(&signature));
    Secret::new(unsigned).map_err(|_| unsignable("the assertion is empty"))
}

fn encoded(value: &Value) -> Result<String, AuthorizerError> {
    serde_json::to_vec(value)
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
        .map_err(|_| unsignable("a part of the assertion cannot be written as JSON"))
}
