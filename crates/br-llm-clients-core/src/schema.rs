use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer};

use crate::build_error::BuildError;
use crate::raw::Raw;

pub const SCHEMA_VERSION: &str = "br-llm-clients-core/2";

#[derive(Debug, PartialEq, thiserror::Error)]
pub(crate) enum LoadError {
    #[error("stored schema {found:?} is not {SCHEMA_VERSION}")]
    Schema { found: String },
    #[error("stored result with status {status:?} {problem}")]
    Result {
        status: &'static str,
        problem: &'static str,
    },
    #[error(transparent)]
    Rule(#[from] BuildError),
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

pub(crate) fn checked_first<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Raw, D::Error> {
    let raw = Raw::deserialize(deserializer)?;
    let Raw::Object(pairs) = &raw else {
        return Err(de::Error::invalid_type(
            Unexpected::Other(raw.kind()),
            &"an object",
        ));
    };
    let mut schemas = pairs.iter().filter(|(key, _)| key == "schema");
    match (schemas.next(), schemas.next()) {
        (None, _) => Err(de::Error::missing_field("schema")),
        (Some(_), Some(_)) => Err(de::Error::duplicate_field("schema")),
        (Some((_, Raw::Text(found))), None) => {
            check(found).map_err(de::Error::custom)?;
            Ok(raw)
        }
        (Some((_, other)), None) => Err(de::Error::invalid_type(
            Unexpected::Other(other.kind()),
            &"the schema as a string",
        )),
    }
}

pub(crate) fn present<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_the_current_schema_when_checked_then_accepted() {
        assert_eq!(check(SCHEMA_VERSION), Ok(()));
    }

    #[test]
    fn given_the_previous_schema_when_checked_then_refused_naming_it() {
        assert_eq!(
            check("br-llm-clients-core/1"),
            Err(LoadError::Schema {
                found: "br-llm-clients-core/1".to_owned()
            })
        );
    }
}
