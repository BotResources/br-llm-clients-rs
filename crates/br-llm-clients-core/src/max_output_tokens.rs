use crate::build_error::BuildError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct MaxOutputTokens(u32);

impl MaxOutputTokens {
    pub fn new(value: u32) -> Result<Self, BuildError> {
        if value == 0 {
            return Err(BuildError::ZeroMaxOutputTokens);
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for MaxOutputTokens {
    type Error = BuildError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> serde::Deserialize<'de> for MaxOutputTokens {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_zero_when_built_then_refused() {
        assert_eq!(
            MaxOutputTokens::new(0),
            Err(BuildError::ZeroMaxOutputTokens)
        );
    }

    #[test]
    fn given_the_largest_32_bit_number_when_built_then_accepted() {
        assert_eq!(MaxOutputTokens::new(u32::MAX).unwrap().get(), u32::MAX);
    }

    #[test]
    fn given_zero_when_loaded_then_refused() {
        assert!(serde_json::from_str::<MaxOutputTokens>("0").is_err());
    }

    #[test]
    fn given_a_number_beyond_32_bits_when_loaded_then_refused() {
        assert!(serde_json::from_str::<MaxOutputTokens>("4294967296").is_err());
    }

    #[test]
    fn given_a_valid_number_when_round_tripped_then_identical() {
        let tokens = MaxOutputTokens::new(8192).unwrap();
        let json = serde_json::to_string(&tokens).unwrap();
        assert_eq!(json, "8192");
        assert_eq!(
            serde_json::from_str::<MaxOutputTokens>(&json).unwrap(),
            tokens
        );
    }
}
