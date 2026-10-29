use std::sync::Arc;

use br_llm_clients_core::{ClientError, ClientRequest};
use br_llm_messages::ResponseDelta;

use crate::normalizer::Normalizer;
use crate::test_decoder::script;
use crate::test_fixtures::user_request;
use crate::test_ids::CountingIds;
use crate::test_stream::kinds;

pub(crate) fn fresh() -> Normalizer {
    fresh_for(&user_request())
}

pub(crate) fn fresh_for(request: &ClientRequest) -> Normalizer {
    Normalizer::new(request, Arc::new(CountingIds::new()), None)
}

pub(crate) fn feed(normalizer: &mut Normalizer, lines: &[&str]) -> Vec<ResponseDelta> {
    script(lines)
        .into_iter()
        .flat_map(|event| normalizer.accept(event).expect("accepted"))
        .collect()
}

fn without_start(deltas: &[ResponseDelta]) -> Vec<String> {
    kinds(deltas)
        .into_iter()
        .filter(|line| !line.starts_with("started"))
        .collect()
}

pub(crate) fn fed(lines: &[&str]) -> Vec<String> {
    without_start(&feed(&mut fresh(), lines))
}

pub(crate) fn steps(normalizer: &mut Normalizer, lines: &[&str]) -> Vec<Vec<String>> {
    script(lines)
        .into_iter()
        .map(|event| without_start(&normalizer.accept(event).expect("accepted")))
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
