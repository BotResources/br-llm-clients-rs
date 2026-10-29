use std::sync::Arc;

use br_llm_clients_common_http::NoAuthorizer;
use br_llm_clients_core::ClientError;
use br_llm_messages::{DeltaOp, ResponseEnding};

use crate::call_spec::CallSpec;
use crate::test_decoder::EndScript;
use crate::test_fixtures::{http_client, spec_with_ids, user_request};
use crate::test_ids::{CountingIds, number_of};
use crate::test_server::closed_endpoint;
use crate::test_stream::{body, dialect_ending, drive_spec};

fn started_blocks(deltas: &[br_llm_messages::ResponseDelta]) -> Vec<u64> {
    deltas
        .iter()
        .filter_map(|delta| match delta.op() {
            DeltaOp::BlockStarted { block, .. } => Some(number_of(*block)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn given_an_id_source_on_the_spec_when_blocks_are_sent_then_their_ids_come_from_it_in_the_order_of_the_blocks()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let ids = Arc::new(CountingIds::from(100));
    let spec = spec_with_ids(&dialect, closed_endpoint(), ids.clone());
    let bytes = body(&[&[
        "start a thinking",
        "start b text",
        "text b later",
        "think a first",
        "end a",
        "end b",
        "start c tool search id1",
        "args c {}",
        "end c",
        "stop tool_use",
    ]]);
    let driven = drive_spec(&spec, &dialect.log, &user_request(), &[bytes]).await;
    assert!(driven.result.is_ok());
    assert_eq!(started_blocks(&driven.sink.deltas), vec![100, 101, 102]);
    assert_eq!(ids.taken(), 3);
}

#[tokio::test]
async fn given_a_judgement_that_fails_after_held_blocks_when_the_stream_ends_then_the_id_source_gave_ids_only_to_the_blocks_that_were_sent()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let ids = Arc::new(CountingIds::from(100));
    let spec = spec_with_ids(&dialect, closed_endpoint(), ids.clone());
    let bytes = body(&[&[
        "start a text",
        "text a hello",
        "start b text",
        "text b held",
        "start c tool search id1",
        "args c {\"q\":",
        "stop exhaust",
    ]]);
    let driven = drive_spec(&spec, &dialect.log, &user_request(), &[bytes]).await;
    assert_eq!(
        driven.result,
        Err(ClientError::ReplyUnusable {
            ending: ResponseEnding::output_exhaustion()
        })
    );
    assert_eq!(started_blocks(&driven.sink.deltas), vec![100]);
    assert_eq!(ids.taken(), 1);
}

#[tokio::test]
async fn given_a_judgement_that_succeeds_when_the_stream_ends_then_the_rehearsal_takes_no_id_from_the_source()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let ids = Arc::new(CountingIds::from(100));
    let spec = spec_with_ids(&dialect, closed_endpoint(), ids.clone());
    let bytes = body(&[&[
        "start a text",
        "text a hello",
        "start b text",
        "text b held",
        "stop finished",
    ]]);
    let driven = drive_spec(&spec, &dialect.log, &user_request(), &[bytes]).await;
    assert!(driven.result.is_ok());
    assert_eq!(started_blocks(&driven.sink.deltas), vec![100, 101]);
    assert_eq!(ids.taken(), 2);
}

#[tokio::test]
async fn given_no_id_source_set_when_blocks_are_sent_then_the_default_gives_distinct_version_7_ids()
{
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let default = CallSpec::new(
        dialect.clone(),
        closed_endpoint(),
        Arc::new(NoAuthorizer),
        http_client(),
    );
    let bytes = body(&[&[
        "start a text",
        "text a one",
        "end a",
        "start b text",
        "text b two",
        "end b",
        "stop finished",
    ]]);
    let driven = drive_spec(&default, &dialect.log, &user_request(), &[bytes]).await;
    let mut seen = Vec::new();
    for delta in &driven.sink.deltas {
        if let DeltaOp::BlockStarted { block, .. } = delta.op() {
            assert_eq!(block.as_uuid().get_version_num(), 7);
            seen.push(*block);
        }
    }
    assert_eq!(seen.len(), 2);
    assert_ne!(seen.first(), seen.get(1));
}
