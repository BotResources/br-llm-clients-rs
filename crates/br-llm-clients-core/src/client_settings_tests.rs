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
fn given_settings_when_stored_then_the_object_has_the_five_keys_and_a_bare_model_string() {
    assert_eq!(
        serde_json::to_value(settings()).unwrap(),
        serde_json::json!({
            "schema": "br-llm-clients-core/2",
            "model": "model-x",
            "reasoning": "off",
            "cache": "not_required",
            "max_output_tokens": 4096
        })
    );
}

#[test]
fn given_settings_of_the_previous_schema_when_loaded_then_refused_naming_the_schema_found() {
    let previous = r#"{"schema":"br-llm-clients-core/1","model":"model-x","reasoning":"off","cache":"not_required","max_output_tokens":4096}"#;
    let error = serde_json::from_str::<ClientSettings>(previous).unwrap_err();
    assert!(error.to_string().contains("br-llm-clients-core/1"));
    let as_value: serde_json::Value = serde_json::from_str(previous).unwrap();
    let error = serde_json::from_value::<ClientSettings>(as_value).unwrap_err();
    assert!(error.to_string().contains("br-llm-clients-core/1"));
}

#[test]
fn given_settings_with_an_unknown_key_when_loaded_then_refused() {
    let mut json = serde_json::to_value(settings()).unwrap();
    json["endpoint"] = serde_json::json!("https://example.org");
    assert!(serde_json::from_value::<ClientSettings>(json).is_err());
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
    for schema in ["br-llm-clients-core/1", "br-llm-clients-core/3", "other/2"] {
        let mut json = serde_json::to_value(settings()).unwrap();
        json["schema"] = serde_json::json!(schema);
        assert!(
            serde_json::from_value::<ClientSettings>(json).is_err(),
            "{schema}"
        );
    }
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

#[test]
fn given_a_blank_model_text_when_built_and_round_tripped_then_it_is_accepted_for_form() {
    let blank = ClientSettings::new(
        ModelName::new("   ").unwrap(),
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(1).unwrap(),
    );
    let json = serde_json::to_value(&blank).unwrap();
    assert_eq!(json["model"], serde_json::json!("   "));
    assert_eq!(
        serde_json::from_value::<ClientSettings>(json).unwrap(),
        blank
    );
}
