use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;
use br_llm_messages::ResponseEnding;

use crate::secret_log::SecretLog;
use crate::test_decoder::EndScript;
use crate::test_fixtures::{failure_of, spec_for, user_request};
use crate::test_server::{TestServer, http};
use crate::test_stream::{body, dialect_ending};

const LISTED: &str = "LISTED-9911";
const LOGGED: &str = "LOGGED-4422";

async fn unusable_reason(reason: String) -> String {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let line = format!("stop other:{reason}");
    let server = TestServer::start(move |_| http("200 OK", &[], &body(&[&[line.as_str()]]))).await;
    let log = SecretLog::new();
    log.record(&Secret::new(LOGGED).expect("secret"));
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new(LISTED).expect("secret")])
        .with_secret_log(log);
    match failure_of(&spec, &user_request()).await {
        ClientError::ReplyUnusable {
            ending: ResponseEnding::Unclassified { reason, .. },
        } => reason,
        other => panic!("not an unusable reply: {other:?}"),
    }
}

#[tokio::test]
async fn given_an_unclassified_ending_whose_code_holds_a_listed_and_a_logged_secret_when_run_then_both_are_replaced()
 {
    let reason = unusable_reason(format!("code-{LISTED}-{LOGGED}-end")).await;
    assert_eq!(reason, "code-[redacted]-[redacted]-end");
}

#[tokio::test]
async fn given_an_unclassified_code_over_64_kib_with_a_secret_at_the_cut_when_run_then_the_replacement_comes_first()
 {
    let listed = unusable_reason(format!("{}{LISTED}", "a".repeat(65_530))).await;
    assert_eq!(
        listed,
        format!("{}[redacted]", "a".repeat(65_530))[..65_536]
    );
    let logged = unusable_reason(format!("{}{LOGGED}", "a".repeat(65_530))).await;
    assert_eq!(
        logged,
        format!("{}[redacted]", "a".repeat(65_530))[..65_536]
    );
}
