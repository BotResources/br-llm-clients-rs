use serde_json::Value;

use crate::cache_ttl::CacheTtl;
use crate::options_error::OptionsError;

const TTL: &str = "ttl";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MessagesCacheOptions {
    ttl: CacheTtl,
}

impl MessagesCacheOptions {
    pub fn from_json(value: &Value) -> Result<Self, OptionsError> {
        let Value::Object(members) = value else {
            return Err(OptionsError::InvalidCacheOption { member: None });
        };
        if let Some(unknown) = members.keys().find(|member| member.as_str() != TTL) {
            return Err(OptionsError::InvalidCacheOption {
                member: Some(unknown.clone()),
            });
        }
        let ttl = match members.get(TTL) {
            None => CacheTtl::default(),
            Some(given) => given.as_str().and_then(CacheTtl::parse).ok_or_else(|| {
                OptionsError::InvalidCacheOption {
                    member: Some(TTL.to_owned()),
                }
            })?,
        };
        Ok(Self { ttl })
    }

    pub fn ttl(&self) -> CacheTtl {
        self.ttl
    }
}
