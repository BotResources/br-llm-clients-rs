fn json_text_or_null(value: Option<&str>) -> String {
    value
        .and_then(|text| serde_json::to_string(text).ok())
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn error_envelope(
    message: Option<&str>,
    param: Option<&str>,
    code: Option<&str>,
) -> String {
    format!(
        "{{\"error\":{{\"message\":{},\"type\":null,\"param\":{},\"code\":{}}}}}",
        json_text_or_null(message),
        json_text_or_null(param),
        json_text_or_null(code),
    )
}
