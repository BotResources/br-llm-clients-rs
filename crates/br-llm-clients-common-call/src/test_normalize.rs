use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::StreamEvent;

use crate::normalizer::Normalizer;
use crate::test_decoder::script;
use crate::test_fixtures::user_request;
use crate::test_stream::kinds;

pub(crate) fn fresh() -> Normalizer {
    Normalizer::new(&user_request())
}

pub(crate) fn fresh_for(request: &ClientRequest) -> Normalizer {
    Normalizer::new(request)
}

pub(crate) fn feed(normalizer: &mut Normalizer, lines: &[&str]) -> Vec<StreamEvent> {
    script(lines)
        .into_iter()
        .flat_map(|event| normalizer.accept(event).expect("accepted"))
        .collect()
}

pub(crate) fn fed(lines: &[&str]) -> Vec<String> {
    kinds(&feed(&mut fresh(), lines))
}

pub(crate) fn steps(normalizer: &mut Normalizer, lines: &[&str]) -> Vec<Vec<String>> {
    script(lines)
        .into_iter()
        .map(|event| kinds(&normalizer.accept(event).expect("accepted")))
        .collect()
}

pub(crate) fn refusal(setup: &[&str], last: &str) -> ClientError {
    let mut normalizer = fresh();
    feed(&mut normalizer, setup);
    let event = script(&[last]).remove(0);
    normalizer.accept(event).expect_err("a refusal")
}

pub(crate) fn is_malformed(error: &ClientError) -> bool {
    matches!(error, ClientError::MalformedResponse { .. })
}
