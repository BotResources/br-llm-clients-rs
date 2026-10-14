use std::fmt;

use crate::secret::REDACTION;
use crate::texts::bounded;

#[derive(Default)]
pub(crate) struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub(crate) fn new<'a>(values: impl IntoIterator<Item = &'a str>) -> Self {
        let mut secrets: Vec<String> = values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect();
        secrets.sort_by_key(|value| std::cmp::Reverse(value.len()));
        secrets.dedup();
        Self { secrets }
    }

    pub(crate) fn clean(&self, text: &str) -> String {
        let mut cleaned = text.to_owned();
        for secret in &self.secrets {
            if cleaned.contains(secret.as_str()) {
                cleaned = cleaned.replace(secret.as_str(), REDACTION);
            }
        }
        cleaned
    }

    pub(crate) fn finish(&self, text: &str) -> String {
        self.clean(&bounded(text))
    }
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Redactor")
            .field("secrets", &self.secrets.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_secret_inside_a_text_when_cleaning_then_it_is_replaced() {
        let redactor = Redactor::new(["sk-123"]);
        assert_eq!(
            redactor.clean("bad sk-123 value sk-123"),
            "bad [redacted] value [redacted]"
        );
    }

    #[test]
    fn given_a_secret_that_contains_another_when_cleaning_then_the_longest_goes_first() {
        let redactor = Redactor::new(["abc", "abcdef"]);
        assert_eq!(redactor.clean("x abcdef y"), "x [redacted] y");
    }

    #[test]
    fn given_an_empty_value_when_building_then_it_is_ignored() {
        let redactor = Redactor::new([""]);
        assert_eq!(redactor.clean("text"), "text");
    }

    #[test]
    fn given_a_redactor_when_debugging_then_the_secrets_are_not_shown() {
        let redactor = Redactor::new(["sk-9"]);
        assert!(!format!("{redactor:?}").contains("sk-9"));
    }
}
