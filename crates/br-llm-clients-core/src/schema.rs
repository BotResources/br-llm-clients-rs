use crate::build_error::BuildError;

pub const SCHEMA_VERSION: &str = "br-llm-clients-core/1";

#[derive(Debug, PartialEq, thiserror::Error)]
pub(crate) enum LoadError {
    #[error("stored schema {found:?} is not br-llm-clients-core/1")]
    Schema { found: String },
    #[error(transparent)]
    Rule(#[from] BuildError),
}

pub(crate) fn current() -> String {
    SCHEMA_VERSION.to_owned()
}

pub(crate) fn check(found: &str) -> Result<(), LoadError> {
    if found == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(LoadError::Schema {
            found: found.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_the_current_schema_when_checked_then_accepted() {
        assert_eq!(check(&current()), Ok(()));
    }

    #[test]
    fn given_another_schema_when_checked_then_refused_naming_it() {
        assert_eq!(
            check("br-llm-clients-core/2"),
            Err(LoadError::Schema {
                found: "br-llm-clients-core/2".to_owned()
            })
        );
    }
}
