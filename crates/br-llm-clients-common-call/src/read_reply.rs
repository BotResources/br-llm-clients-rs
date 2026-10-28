use br_llm_clients_common_http::Response;
use br_llm_clients_core::{ClientError, ClientRequest, StreamSink};
use br_llm_messages::ModelResponse;

use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;
use crate::failure_map::body_failure;
use crate::reader::{Flow, Reader};
use crate::secrets_cache::SecretsCache;

pub(crate) async fn read_reply(
    spec: &CallSpec,
    secrets: &SecretsCache,
    request: &ClientRequest,
    response: Response,
    sink: &mut dyn StreamSink,
    record: &mut CallRecord,
) -> Result<ModelResponse, ClientError> {
    let (_, _, mut body) = response.into_parts();
    let mut reader = Reader::new(spec, secrets, request, sink, record);
    loop {
        match body.next_chunk().await {
            Err(error) => return Err(body_failure(&error)),
            Ok(None) => {
                reader.end_of_body().await?;
                break;
            }
            Ok(Some(chunk)) => {
                if let Flow::Complete = reader.chunk(&chunk).await? {
                    break;
                }
            }
        }
    }
    reader.conclude().await
}
