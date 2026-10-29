use br_llm_clients_common_http::{
    AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError, BoxFuture, Secret,
};

use crate::secret_log::SecretLog;

pub(crate) struct RecordingAuthorizer {
    pub(crate) log: SecretLog,
    pub(crate) token: &'static str,
}

impl Authorizer for RecordingAuthorizer {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        Box::pin(async move {
            let token = Secret::new(self.token).expect("token");
            self.log.record(&token);
            Ok(vec![AuthorizedHeader::new("x-token", token)])
        })
    }
}
