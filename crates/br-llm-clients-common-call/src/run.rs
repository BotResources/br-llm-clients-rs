use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, StreamSink, render_history};
use br_llm_messages::ModelResponse;

use crate::answered_model::answered_model;
use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;
use crate::failure_map::send_failure;
use crate::outcome::outcome;
use crate::read_reply::read_reply;
use crate::refusal::refuse;
use crate::request_build::build_request;

pub async fn run(
    spec: &CallSpec,
    request: &ClientRequest,
    sink: &mut dyn StreamSink,
) -> CallOutcome {
    let mut record = CallRecord::start();
    let result = exchange(spec, request, sink, &mut record).await;
    outcome(&spec.secrets_in_effect(), record, result)
}

async fn exchange(
    spec: &CallSpec,
    request: &ClientRequest,
    sink: &mut dyn StreamSink,
    record: &mut CallRecord,
) -> Result<ModelResponse, ClientError> {
    let rendered = render_history(request)?;
    let encoded = spec.dialect.encode(request, &rendered)?;
    let http_request = build_request(spec, &encoded)?;
    let response = spec
        .http
        .send(http_request, &*spec.authorizer)
        .await
        .map_err(|error| send_failure(&error))?;
    record.request_id = spec.dialect.request_id(response.headers());
    record.answered_model = answered_model(spec, response.headers());
    if (200..300).contains(&response.status()) {
        read_reply(spec, request, response, sink, record).await
    } else {
        Err(refuse(spec, response).await)
    }
}
