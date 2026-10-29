use br_llm_clients_common_auth::{AccessToken, FormTokenClient, TokenForm, TokenSource};
use br_llm_clients_common_call::{HttpClient, Secret, SecretLog};
use br_llm_clients_common_http::{AuthorizerError, BoxFuture};

use crate::assertion;
use crate::service_account_key::ServiceAccountKey;

pub(crate) struct GoogleTokenSource {
    key: ServiceAccountKey,
    client: FormTokenClient,
    log: SecretLog,
}

impl GoogleTokenSource {
    pub(crate) fn new(key: ServiceAccountKey, http: HttpClient, log: SecretLog) -> Self {
        let client = FormTokenClient::new(http, key.token_endpoint().clone());
        Self { key, client, log }
    }
}

impl TokenSource for GoogleTokenSource {
    fn fetch(&self) -> BoxFuture<'_, Result<AccessToken, AuthorizerError>> {
        Box::pin(async move {
            let assertion = assertion::sign(&self.key)?;
            self.log.record(&assertion);
            let reply = self.client.fetch(&TokenForm::jwt_bearer(assertion)).await?;
            Ok(reply.access_token)
        })
    }

    fn secrets(&self) -> Vec<Secret> {
        vec![self.key.private_key().clone()]
    }
}
