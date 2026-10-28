use br_llm_clients_core::{
    Capabilities, ClientError, ClientRequest, ClientSettings, render_history,
};

pub(crate) fn check(
    capabilities: &Capabilities,
    settings: &ClientSettings,
    request: &ClientRequest,
) -> Result<(), ClientError> {
    render_history(request)?;
    capabilities.check(settings, request)
}
