use br_llm_clients_common_http::Secret;

pub(crate) const TEXT_CAP: usize = 65_536;

const PLACEHOLDER: &str = "[redacted]";

#[derive(Clone, Default)]
pub(crate) struct Secrets {
    longest_first: Vec<Secret>,
}

impl Secrets {
    pub(crate) fn new(mut secrets: Vec<Secret>) -> Self {
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.expose().len()));
        Self {
            longest_first: secrets,
        }
    }

    pub(crate) fn replace(&self, text: &str) -> String {
        self.longest_first
            .iter()
            .fold(text.to_owned(), |cleaned, secret| {
                cleaned.replace(secret.expose(), PLACEHOLDER)
            })
    }

    pub(crate) fn clean(&self, text: &str) -> String {
        cut(self.replace(text))
    }
}

pub(crate) fn cut(mut text: String) -> String {
    if text.len() <= TEXT_CAP {
        return text;
    }
    let mut end = TEXT_CAP;
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.truncate(end);
    text
}
