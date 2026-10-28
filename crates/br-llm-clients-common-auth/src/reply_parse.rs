use std::time::Duration;

use br_llm_clients_common_http::Secret;
use serde_json::{Map, Value};

use crate::access_token::AccessToken;
use crate::token_reply::TokenReply;

const MAX_ERROR_CODE_CHARS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flaw {
    NotAnObject,
    AccessToken,
    TokenType,
    ExpiresIn,
    RefreshToken,
}

impl Flaw {
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::NotAnObject => "a body that is not a JSON object",
            Self::AccessToken => "an access_token that is missing or empty",
            Self::TokenType => "a token_type that is not Bearer",
            Self::ExpiresIn => "an expires_in that is not a whole number of seconds",
            Self::RefreshToken => "a refresh_token that is empty or not a text",
        }
    }
}

fn object(bytes: &[u8]) -> Option<Map<String, Value>> {
    let text = std::str::from_utf8(bytes).ok()?;
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Object(members) => Some(members),
        _ => None,
    }
}

pub(crate) fn parse_reply(bytes: &[u8]) -> Result<TokenReply, Flaw> {
    let members = object(bytes).ok_or(Flaw::NotAnObject)?;
    let access_token = non_empty_secret(members.get("access_token")).ok_or(Flaw::AccessToken)?;
    if let Some(token_type) = members.get("token_type") {
        match token_type {
            Value::String(text) if text.eq_ignore_ascii_case("Bearer") => {}
            _ => return Err(Flaw::TokenType),
        }
    }
    let valid_for = match members.get("expires_in") {
        None => None,
        Some(value) => Some(seconds(value).ok_or(Flaw::ExpiresIn)?),
    };
    let refresh_token = match members.get("refresh_token") {
        None => None,
        Some(value) => Some(non_empty_secret(Some(value)).ok_or(Flaw::RefreshToken)?),
    };
    Ok(TokenReply {
        access_token: AccessToken::new(access_token, valid_for),
        refresh_token,
    })
}

pub(crate) fn error_code(bytes: &[u8]) -> Option<String> {
    let members = object(bytes)?;
    let Value::String(code) = members.get("error")? else {
        return None;
    };
    let length = code.chars().count();
    let well_formed = (1..=MAX_ERROR_CODE_CHARS).contains(&length)
        && code.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    well_formed.then(|| code.clone())
}

fn non_empty_secret(value: Option<&Value>) -> Option<Secret> {
    match value? {
        Value::String(text) => Secret::new(text.as_str()).ok(),
        _ => None,
    }
}

fn seconds(value: &Value) -> Option<Duration> {
    match value {
        Value::Number(number) => number.as_u64().map(Duration::from_secs),
        Value::String(text) => digits(text).map(Duration::from_secs),
        _ => None,
    }
}

fn digits(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(text.bytes().fold(0u64, |total, byte| {
        total
            .saturating_mul(10)
            .saturating_add(u64::from(byte.saturating_sub(b'0')))
    }))
}
