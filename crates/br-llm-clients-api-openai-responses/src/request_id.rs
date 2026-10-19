use br_llm_clients_common_call::Headers;

pub(crate) fn request_id(headers: &Headers) -> Option<String> {
    headers
        .get_text("x-request-id")
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
