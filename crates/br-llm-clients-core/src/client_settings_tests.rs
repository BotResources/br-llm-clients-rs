use br_llm_messages::ModelName;

use crate::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel, SCHEMA_VERSION};

fn settings() -> ClientSettings {
    ClientSettings::new(
        ModelName::new("model-x").unwrap(),
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(4096).unwrap(),
    )
}

#[test]
fn given_settings_when_stored_then_they_carry_the_schema() {
    let json = serde_json::to_value(settings()).unwrap();
    assert_eq!(json["schema"], serde_json::json!(SCHEMA_VERSION));
    assert_eq!(SCHEMA_VERSION, "br-llm-clients-core/2");
}

#[test]
fn given_settings_when_round_tripped_then_identical() {
    let json = serde_json::to_value(settings()).unwrap();
    assert_eq!(
        serde_json::from_value::<ClientSettings>(json).unwrap(),
        settings()
    );
}

#[test]
fn given_another_schema_when_loaded_then_refused() {
    let mut json = serde_json::to_value(settings()).unwrap();
    json["schema"] = serde_json::json!("br-llm-clients-core/1");
    assert!(serde_json::from_value::<ClientSettings>(json).is_err());
}

#[test]
fn given_no_schema_when_loaded_then_refused() {
    let mut json = serde_json::to_value(settings()).unwrap();
    json.as_object_mut().unwrap().remove("schema");
    assert!(serde_json::from_value::<ClientSettings>(json).is_err());
}

#[test]
fn given_a_missing_field_when_loaded_then_refused_because_no_field_has_a_default() {
    for field in ["model", "reasoning", "cache", "max_output_tokens"] {
        let mut json = serde_json::to_value(settings()).unwrap();
        json.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ClientSettings>(json).is_err(),
            "{field}"
        );
    }
}

#[test]
fn given_stored_settings_breaking_a_rule_when_loaded_then_refused() {
    let base = serde_json::to_value(settings()).unwrap();
    for (field, bad) in [
        ("model", serde_json::json!("")),
        ("max_output_tokens", serde_json::json!(0)),
        ("reasoning", serde_json::json!("ultra")),
        ("cache", serde_json::json!("sometimes")),
    ] {
        let mut json = base.clone();
        json[field] = bad;
        assert!(
            serde_json::from_value::<ClientSettings>(json).is_err(),
            "{field}"
        );
    }
}

#[test]
fn given_settings_when_read_then_the_four_fields_come_back_as_given() {
    let settings = settings();
    assert_eq!(settings.model().as_str(), "model-x");
    assert_eq!(settings.reasoning(), ReasoningLevel::Off);
    assert_eq!(settings.cache(), CacheRequirement::NotRequired);
    assert_eq!(settings.max_output_tokens().get(), 4096);
}
