const MAX_NAME_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AuthConfigError {
    #[error("invalid form field: {text}")]
    InvalidFormField { text: String },
    #[error("duplicate form field: {text}")]
    DuplicateFormField { text: String },
}

impl AuthConfigError {
    pub(crate) fn empty_name() -> Self {
        Self::InvalidFormField {
            text: "a form field has an empty name".to_owned(),
        }
    }

    pub(crate) fn empty_value(name: &str) -> Self {
        Self::InvalidFormField {
            text: format!("the form field `{}` is empty", label(name)),
        }
    }

    pub(crate) fn duplicate(name: &str) -> Self {
        Self::DuplicateFormField {
            text: format!("the form field `{}` is given twice", label(name)),
        }
    }
}

fn label(name: &str) -> String {
    let mut shown: String = name
        .chars()
        .take(MAX_NAME_CHARS)
        .flat_map(char::escape_debug)
        .collect();
    if name.chars().nth(MAX_NAME_CHARS).is_some() {
        shown.push_str("...");
    }
    shown
}
