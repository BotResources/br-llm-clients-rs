use std::fmt;

use br_llm_clients_common_http::Secret;

use crate::error::AuthConfigError;

#[derive(Clone)]
pub(crate) enum FieldValue {
    Plain(String),
    Secret(Secret),
}

#[derive(Clone, Default)]
pub struct TokenForm {
    pub(crate) fields: Vec<(String, FieldValue)>,
}

impl TokenForm {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn plain(self, name: &str, value: &str) -> Result<Self, AuthConfigError> {
        self.add(name, FieldValue::Plain(value.to_owned()))
    }

    pub fn secret(self, name: &str, value: Secret) -> Result<Self, AuthConfigError> {
        self.add(name, FieldValue::Secret(value))
    }

    pub fn client_credentials(client_id: &str, secret: Secret) -> Result<Self, AuthConfigError> {
        if client_id.is_empty() {
            return Err(AuthConfigError::empty_value("client_id"));
        }
        Ok(Self::fixed("grant_type", "client_credentials")
            .fixed_plain("client_id", client_id)
            .fixed_secret("client_secret", secret))
    }

    pub fn jwt_bearer(assertion: Secret) -> Self {
        Self::fixed("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer")
            .fixed_secret("assertion", assertion)
    }

    pub fn refresh_token(token: Secret) -> Self {
        Self::fixed("grant_type", "refresh_token").fixed_secret("refresh_token", token)
    }

    pub fn secrets(&self) -> Vec<Secret> {
        self.fields
            .iter()
            .filter_map(|(_, value)| match value {
                FieldValue::Secret(secret) => Some(secret.clone()),
                FieldValue::Plain(_) => None,
            })
            .collect()
    }

    pub(crate) fn echoes(&self, text: &str) -> bool {
        self.fields.iter().any(|(_, value)| match value {
            FieldValue::Plain(plain) => plain == text,
            FieldValue::Secret(secret) => secret.appears_in(text) || secret.expose().contains(text),
        })
    }

    fn add(mut self, name: &str, value: FieldValue) -> Result<Self, AuthConfigError> {
        if name.is_empty() {
            return Err(AuthConfigError::empty_name());
        }
        if self.fields.iter().any(|(existing, _)| existing == name) {
            return Err(AuthConfigError::duplicate(name));
        }
        self.fields.push((name.to_owned(), value));
        Ok(self)
    }

    fn fixed(name: &str, value: &str) -> Self {
        Self::new().fixed_plain(name, value)
    }

    fn fixed_plain(mut self, name: &str, value: &str) -> Self {
        self.fields
            .push((name.to_owned(), FieldValue::Plain(value.to_owned())));
        self
    }

    fn fixed_secret(mut self, name: &str, value: Secret) -> Self {
        self.fields
            .push((name.to_owned(), FieldValue::Secret(value)));
        self
    }
}

impl fmt::Debug for TokenForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.fields.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("TokenForm").field("fields", &names).finish()
    }
}
