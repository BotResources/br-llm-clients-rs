use br_llm_clients_core::{Capabilities, ClientError, ClientRequest, ClientSettings};

pub(crate) fn check(
    capabilities: &Capabilities,
    settings: &ClientSettings,
    request: &ClientRequest,
) -> Result<(), ClientError> {
    request.check_messages()?;
    capabilities.check(settings, request)
}
