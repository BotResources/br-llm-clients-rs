use std::time::Duration;

use rcgen::{CertificateParams, KeyPair};

use crate::client::HttpClient;
use crate::config::HttpConfig;
use crate::config_error::ConfigError;

const SECOND: Duration = Duration::from_secs(1);

fn base() -> HttpConfig {
    HttpConfig::new(SECOND, SECOND, SECOND)
}

fn invalid(config: HttpConfig) -> String {
    match HttpClient::new(config) {
        Err(ConfigError::InvalidConfig { text }) => text,
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

fn certificate_pem() -> String {
    let key = KeyPair::generate().expect("key");
    CertificateParams::new(vec!["localhost".to_owned()])
        .expect("params")
        .self_signed(&key)
        .expect("certificate")
        .pem()
}

#[test]
fn given_only_the_three_timeouts_when_building_then_the_client_is_built() {
    assert!(HttpClient::new(base()).is_ok());
}

#[test]
fn given_a_zero_timeout_when_building_then_it_is_an_invalid_config_that_names_the_field() {
    let mut zero_connect = base();
    zero_connect.connect_timeout = Duration::ZERO;
    assert!(invalid(zero_connect).contains("connect_timeout"));
    let mut zero_headers = base();
    zero_headers.headers_timeout = Duration::ZERO;
    assert!(invalid(zero_headers).contains("headers_timeout"));
    let mut zero_idle = base();
    zero_idle.idle_timeout = Duration::ZERO;
    assert!(invalid(zero_idle).contains("idle_timeout"));
    let mut zero_total = base();
    zero_total.total_timeout = Some(Duration::ZERO);
    assert!(invalid(zero_total).contains("total_timeout"));
}

#[test]
fn given_a_huge_timeout_when_building_then_the_client_is_built() {
    let mut config = base();
    config.total_timeout = Some(Duration::MAX);
    config.connect_timeout = Duration::MAX;
    assert!(HttpClient::new(config).is_ok());
}

#[test]
fn given_no_user_agent_when_building_then_the_default_names_the_library_and_its_version() {
    let value = base().validate().expect("valid");
    assert_eq!(
        value.to_str().expect("text"),
        "br-llm-clients-common-http/0.1.0"
    );
}

#[test]
fn given_a_user_agent_when_building_then_it_replaces_the_default() {
    let mut config = base();
    config.user_agent = Some("my-app/2".to_owned());
    assert_eq!(
        config.validate().expect("valid").to_str().expect("text"),
        "my-app/2"
    );
}

#[test]
fn given_an_empty_or_invalid_user_agent_when_building_then_it_is_an_invalid_config() {
    for agent in ["", "a\nb", "tail\n"] {
        let mut config = base();
        config.user_agent = Some(agent.to_owned());
        assert!(invalid(config).contains("user_agent"), "{agent:?}");
    }
}

#[test]
fn given_a_valid_pem_root_when_building_then_the_client_is_built() {
    let mut config = base();
    config.extra_root_certificates = vec![certificate_pem(), certificate_pem()];
    assert!(HttpClient::new(config).is_ok());
}

#[test]
fn given_texts_that_are_not_certificates_when_building_then_it_is_an_invalid_config_without_the_text()
 {
    let bad_der = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
    let bad_base64 = "-----BEGIN CERTIFICATE-----\n!!!!\n-----END CERTIFICATE-----\n";
    let only_key = "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n";
    for text in [
        "",
        "not pem at all SECRET-PEM-TEXT",
        bad_der,
        bad_base64,
        only_key,
    ] {
        let mut config = base();
        config.extra_root_certificates = vec![certificate_pem(), text.to_owned()];
        let reason = invalid(config);
        assert!(reason.contains("extra_root_certificates[1]"), "{reason}");
        assert!(!reason.contains("SECRET-PEM-TEXT"), "{reason}");
    }
}
