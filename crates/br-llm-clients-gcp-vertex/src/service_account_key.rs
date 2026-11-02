use std::fmt;
use std::sync::Arc;

use br_llm_clients_common_call::{Endpoint, Secret};
use ring::signature::RsaKeyPair;
use rustls_pki_types::PrivatePkcs8KeyDer;
use rustls_pki_types::pem::PemObject;

use crate::build_error::VertexBuildError;
use crate::key_fields::KeyFields;

const KEY_TYPE: &str = "service_account";
const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_UNIVERSE: &str = "googleapis.com";

#[derive(Clone)]
pub struct ServiceAccountKey {
    client_email: String,
    private_key: Secret,
    private_key_id: Option<String>,
    token_uri: String,
    token_endpoint: Endpoint,
    key_pair: Arc<RsaKeyPair>,
}

impl ServiceAccountKey {
    pub fn from_json(text: &str) -> Result<Self, VertexBuildError> {
        let fields = KeyFields::parse(text)?;
        if fields.optional("type")? != Some(KEY_TYPE) {
            return Err(VertexBuildError::credential(
                "the member `type` of the service account key is not `service_account`",
            ));
        }
        if fields
            .optional("universe_domain")?
            .is_some_and(|universe| universe != DEFAULT_UNIVERSE)
        {
            return Err(VertexBuildError::credential(
                "the service account key belongs to a universe other than googleapis.com",
            ));
        }
        let client_email = fields.required("client_email")?.to_owned();
        let pem = fields.required("private_key")?;
        let key_pair = Arc::new(key_pair(pem)?);
        let private_key = Secret::new(pem).map_err(|_| {
            VertexBuildError::credential(
                "the member `private_key` of the service account key is empty",
            )
        })?;
        let private_key_id = fields
            .optional("private_key_id")?
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        let token_uri = fields
            .optional("token_uri")?
            .unwrap_or(DEFAULT_TOKEN_URI)
            .to_owned();
        let token_endpoint = Endpoint::https(&token_uri).map_err(|_| {
            VertexBuildError::credential(
                "the member `token_uri` of the service account key is not a valid https URL",
            )
        })?;
        Ok(Self {
            client_email,
            private_key,
            private_key_id,
            token_uri,
            token_endpoint,
            key_pair,
        })
    }

    pub(crate) fn client_email(&self) -> &str {
        &self.client_email
    }

    pub(crate) fn private_key(&self) -> &Secret {
        &self.private_key
    }

    pub(crate) fn private_key_id(&self) -> Option<&str> {
        self.private_key_id.as_deref()
    }

    pub(crate) fn token_uri(&self) -> &str {
        &self.token_uri
    }

    pub(crate) fn token_endpoint(&self) -> &Endpoint {
        &self.token_endpoint
    }

    pub(crate) fn key_pair(&self) -> &RsaKeyPair {
        &self.key_pair
    }
}

fn key_pair(pem: &str) -> Result<RsaKeyPair, VertexBuildError> {
    let not_rsa = || {
        VertexBuildError::credential(
            "the member `private_key` of the service account key is not an RSA private key in PKCS#8 PEM",
        )
    };
    let der = PrivatePkcs8KeyDer::from_pem_slice(pem.as_bytes()).map_err(|_| not_rsa())?;
    RsaKeyPair::from_pkcs8(der.secret_pkcs8_der()).map_err(|_| not_rsa())
}

impl fmt::Debug for ServiceAccountKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceAccountKey")
            .field("client_email", &self.client_email)
            .field("private_key_id", &self.private_key_id)
            .finish_non_exhaustive()
    }
}
