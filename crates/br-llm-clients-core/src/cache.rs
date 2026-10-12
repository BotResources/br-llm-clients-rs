#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CacheRequirement {
    Required,
    NotRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_both_values_when_round_tripped_then_identical() {
        for value in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            let json = serde_json::to_value(value).unwrap();
            assert_eq!(
                serde_json::from_value::<CacheRequirement>(json).unwrap(),
                value
            );
        }
    }

    #[test]
    fn given_a_value_unknown_to_this_version_when_loaded_then_refused() {
        assert!(serde_json::from_str::<CacheRequirement>("\"sometimes\"").is_err());
    }
}
