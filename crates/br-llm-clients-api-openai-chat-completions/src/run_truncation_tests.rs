use br_llm_clients_common_call::run;
use br_llm_clients_core::ClientError;

use crate::run_support::{ok_stream, spec_for};
use crate::test_server::TestServer;
use crate::test_sink::RecordingSink;
use crate::test_support::{dialect, hello};

fn full_stream() -> Vec<u8> {
    [
        r#"{"model":"m","choices":[{"delta":{"content":"Hi"}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"f","arguments":"{}"}}]}}]}"#,
        r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":1,"completion_tokens":2}}"#,
    ]
    .iter()
    .flat_map(|data| format!("data: {data}\n\n").into_bytes())
    .chain(b"data: [DONE]\n\n".iter().copied())
    .collect()
}

#[tokio::test]
async fn given_the_stream_cut_at_every_byte_when_called_then_each_cut_ends_as_truncated_or_malformed()
 {
    let full = full_stream();
    let served = full.clone();
    let server = TestServer::start(move |index| {
        let end = index.min(served.len());
        ok_stream(served.get(..end).unwrap_or_default())
    })
    .await;
    let spec = spec_for(dialect(), &server);
    for cut in 0..=full.len() {
        let mut sink = RecordingSink::default();
        let outcome = run(&spec, &hello(), &mut sink).await;
        match outcome.result() {
            Ok(_) => assert_eq!(cut, full.len(), "only the whole stream succeeds"),
            Err(error) => assert!(
                matches!(
                    error,
                    ClientError::StreamTruncated | ClientError::MalformedResponse { .. }
                ),
                "cut {cut}: {error:?}"
            ),
        }
    }
}
