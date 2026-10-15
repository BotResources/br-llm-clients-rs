use std::future::Future;
use std::sync::Arc;

use tokio::time::{Instant, timeout_at};

use crate::authorize::merge;
use crate::authorizer::{AuthorizationRequest, Authorizer};
use crate::client::Shared;
use crate::exchange::exchange;
use crate::prepare::prepare;
use crate::redact::Redactor;
use crate::request::Request;
use crate::response::Response;
use crate::send_error::SendError;

pub(crate) async fn run(
    shared: &Arc<Shared>,
    request: Request,
    authorizer: &dyn Authorizer,
    total_deadline: Option<Instant>,
) -> Result<Response, SendError> {
    let mut prepared = prepare(request, &shared.user_agent)?;
    let returned = {
        let view = AuthorizationRequest::new(
            &prepared.path_and_query,
            &prepared.host,
            &prepared.headers,
            &prepared.body,
        );
        let waiting = within(total_deadline, authorizer.authorize(&view)).await;
        match waiting {
            Some(result) => result.map_err(|error| SendError::Authorizer(error.capped()))?,
            None => return Err(total_timeout(shared, &Redactor::default())),
        }
    };
    let redactor = Arc::new(merge(&mut prepared, returned)?);
    let sending = exchange(shared, &prepared, &redactor, total_deadline);
    match within(total_deadline, sending).await {
        Some(result) => result,
        None => Err(total_timeout(shared, &redactor)),
    }
}

async fn within<F: Future>(deadline: Option<Instant>, work: F) -> Option<F::Output> {
    match deadline {
        None => Some(work.await),
        Some(at) => timeout_at(at, work).await.ok(),
    }
}

fn total_timeout(shared: &Shared, redactor: &Redactor) -> SendError {
    let limit = shared.config.total_timeout.unwrap_or_default();
    SendError::TotalTimeout {
        text: redactor.finish(&format!(
            "no response headers within the total timeout of {limit:?}"
        )),
    }
}
