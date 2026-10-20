pub(crate) fn is_visible_text(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| (0x21..=0x7E).contains(&byte))
}
