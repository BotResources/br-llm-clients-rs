pub(crate) fn is_visible(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}
