pub(super) fn text(value: &str) -> String {
    escape(value, false)
}

pub(super) fn label(value: &str) -> String {
    escape(value, true)
}

fn escape(value: &str, quotes: bool) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if quotes => escaped.push_str("&quot;"),
            other => escaped.push(other),
        }
    }
    escaped
}
