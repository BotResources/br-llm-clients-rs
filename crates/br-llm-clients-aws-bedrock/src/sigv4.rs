use std::sync::Arc;
use std::time::SystemTime;

use aws_credential_types::Credentials;
use aws_sigv4::http_request::{
    SignableBody, SignableRequest, SigningParams, SigningSettings, sign,
};
use aws_sigv4::sign::v4;
use br_llm_clients_common_call::{Authorizer, Secret, SecretLog};
use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, AuthorizerError, BoxFuture,
};
use tracing::Dispatch;
use tracing::dispatcher::with_default;

use crate::aws_credentials::{AwsCredentials, CredentialSource};
use crate::visible::is_visible;

const SERVICE_NAME: &str = "bedrock";
const PROVIDER_NAME: &str = "br-llm-clients-aws-bedrock";

pub(crate) struct Sigv4Authorizer {
    source: Arc<dyn CredentialSource>,
    region: String,
    log: SecretLog,
}

impl Sigv4Authorizer {
    pub(crate) fn new(source: Arc<dyn CredentialSource>, region: String, log: SecretLog) -> Self {
        Self {
            source,
            region,
            log,
        }
    }

    fn record(&self, credentials: &AwsCredentials) {
        self.log.record(credentials.secret_access_key());
        if let Some(token) = credentials.session_token() {
            self.log.record(token);
        }
    }
}

impl Authorizer for Sigv4Authorizer {
    fn authorize<'a>(
        &'a self,
        request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move {
            let credentials = self.source.credentials().await?;
            self.record(&credentials);
            signed_headers(&credentials, &self.region, request)
        })
    }
}

fn unusable_credential(reason: &str) -> AuthorizerError {
    AuthorizerError::unavailable(format!("the credentials cannot sign: {reason}"))
}

fn unsignable(reason: &str) -> AuthorizerError {
    AuthorizerError::unavailable(format!("the request cannot be signed: {reason}"))
}

fn signed_headers(
    credentials: &AwsCredentials,
    region: &str,
    request: &AuthorizationRequest<'_>,
) -> Result<Vec<AuthorizedHeader>, AuthorizerError> {
    if !is_visible(credentials.access_key_id()) {
        return Err(unusable_credential(
            "the access key id is empty or is not visible ASCII text",
        ));
    }
    let headers = request
        .headers()
        .map(|(name, value)| std::str::from_utf8(value).map(|text| (name, text)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| unsignable("a header value of the request is not text"))?;
    let identity = Credentials::new(
        credentials.access_key_id(),
        credentials.secret_access_key().expose(),
        credentials
            .session_token()
            .map(|token| token.expose().to_owned()),
        None,
        PROVIDER_NAME,
    )
    .into();
    let params: SigningParams<'_> = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name(SERVICE_NAME)
        .time(SystemTime::now())
        .settings(SigningSettings::default())
        .build()
        .map_err(|_| unsignable("the signing parameters are incomplete"))?
        .into();
    let signable = SignableRequest::new(
        request.method(),
        format!("https://{}{}", request.host(), request.path_and_query()),
        headers.into_iter(),
        SignableBody::Bytes(request.body()),
    )
    .map_err(|_| unsignable("the URI of the request is not valid"))?;
    let output = with_default(&Dispatch::none(), || sign(signable, &params))
        .map_err(|_| unsignable("the signer refused the request"))?;
    let (instructions, _signature) = output.into_parts();
    let (signing_headers, query) = instructions.into_parts();
    if !query.is_empty() {
        return Err(unsignable("the signer asked for query parameters"));
    }
    signing_headers
        .into_iter()
        .map(|header| {
            Secret::new(header.value())
                .map(|value| AuthorizedHeader::new(header.name(), value))
                .map_err(|_| unsignable("the signer gave an empty header value"))
        })
        .collect()
}
