#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningLevel {
    Off,
    Low,
    Medium,
    High,
    Extra,
    Max,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASCENDING: [ReasoningLevel; 6] = [
        ReasoningLevel::Off,
        ReasoningLevel::Low,
        ReasoningLevel::Medium,
        ReasoningLevel::High,
        ReasoningLevel::Extra,
        ReasoningLevel::Max,
    ];

    #[test]
    fn given_the_six_levels_when_compared_then_they_rise_from_off_to_max() {
        for pair in ASCENDING.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }

    #[test]
    fn given_every_level_when_round_tripped_then_identical() {
        for level in ASCENDING {
            let json = serde_json::to_value(level).unwrap();
            assert_eq!(
                serde_json::from_value::<ReasoningLevel>(json).unwrap(),
                level
            );
        }
    }

    #[test]
    fn given_a_level_unknown_to_this_version_when_loaded_then_refused() {
        assert!(serde_json::from_str::<ReasoningLevel>("\"ultra\"").is_err());
    }
}
