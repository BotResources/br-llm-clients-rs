use br_llm_clients_common_http::Secret;

pub(crate) const TEXT_CAP: usize = 65_536;

const PLACEHOLDER: &str = "[redacted]";

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
                if secret.appears_in(&cleaned) {
                    cleaned.replace(secret.expose(), PLACEHOLDER)
                } else {
                    cleaned
                }
            })
    }

    pub(crate) fn clean(&self, text: &str) -> String {
        cut(self.replace(text))
    }

    pub(crate) fn appears_in(&self, text: &str) -> bool {
        self.longest_first
            .iter()
            .any(|secret| secret.appears_in(text))
    }

    pub(crate) fn trim_cut(&self, bytes: &mut Vec<u8>) {
        let room = bytes.len().saturating_add(1);
        let trimmed = self
            .longest_first
            .iter()
            .filter_map(|secret| {
                let secret = secret.expose().as_bytes();
                (1..secret.len().min(room))
                    .rev()
                    .find(|&start| secret.get(..start).is_some_and(|s| bytes.ends_with(s)))
            })
            .max();
        if let Some(trimmed) = trimmed {
            bytes.truncate(bytes.len().saturating_sub(trimmed));
        }
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
