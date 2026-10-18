use br_llm_clients_common_http::Response;
use br_llm_clients_core::{ClientError, ClientRequest, StreamSink};
use br_llm_messages::Step;

use crate::call_record::CallRecord;
use crate::call_spec::CallSpec;

pub(crate) async fn read_reply(
    _spec: &CallSpec,
    _request: &ClientRequest,
    _response: Response,
    _sink: &mut dyn StreamSink,
    _record: &mut CallRecord,
) -> Result<Step, ClientError> {
    Err(ClientError::StreamTruncated)
}
