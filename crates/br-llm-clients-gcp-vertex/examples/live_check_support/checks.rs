use br_llm_clients_core::{CallOutcome, Client, ClientError, ClientRequest, OutputMode};
use br_llm_clients_gcp_vertex::VertexClient;
use br_llm_messages::ModelResponse;

use crate::report::{has_text, print_request_id, variant};
use crate::sink::CountingSink;
use crate::thread::Thread;

const SHORT: &str = "You answer in one short sentence.";

pub async fn call(
    client: &VertexClient,
    request: &ClientRequest,
    sink: &mut CountingSink,
) -> CallOutcome {
    let outcome = client.call(request, sink).await;
    print_request_id(&outcome);
    outcome
}

pub async fn answered(
    client: &VertexClient,
    request: &ClientRequest,
    sink: &mut CountingSink,
) -> Result<ModelResponse, String> {
    let (_, result) = call(client, request, sink).await.into_parts();
    result.map_err(|error| format!("the call failed with {}", variant(&error)))
}

pub fn require(condition: bool, reason: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(reason.to_owned())
    }
}

fn question(text: &str) -> Result<ClientRequest, String> {
    let mut thread = Thread::new()?;
    thread.say(text)?;
    thread.request(Some(SHORT), Vec::new(), OutputMode::Plain)
}

pub async fn plain(client: &VertexClient) -> Result<(), String> {
    let request = question("What is the capital of France?")?;
    let response = answered(client, &request, &mut CountingSink::default()).await?;
    require(has_text(&response), "the reply holds no text")
}

pub async fn streaming(client: &VertexClient) -> Result<(), String> {
    let request = question("Name three rivers of Europe.")?;
    let mut sink = CountingSink::default();
    let response = answered(client, &request, &mut sink).await?;
    println!(
        "  streamed deltas: {}, of which text deltas: {}",
        sink.deltas(),
        sink.text_deltas()
    );
    require(sink.text_deltas() > 0, "no text delta arrived")?;
    require(has_text(&response), "the reply holds no text")
}

pub async fn usage(client: &VertexClient) -> Result<(), String> {
    let request = question("Say hello.")?;
    let outcome = call(client, &request, &mut CountingSink::default()).await;
    let usage = outcome.report().usage().cloned();
    if let Err(error) = outcome.result() {
        return Err(format!("the call failed with {}", variant(error)));
    }
    let usage = usage.ok_or("the report holds no usage")?;
    let input = usage.input_tokens().unwrap_or(0);
    let output = usage.output_tokens().unwrap_or(0);
    println!("  input tokens: {input}, output tokens: {output}");
    require(input > 0, "the input token count is zero or missing")?;
    require(output > 0, "the output token count is zero or missing")
}

pub async fn missing_model(client: &VertexClient) -> Result<(), String> {
    let request = question("Say hello.")?;
    match call(client, &request, &mut CountingSink::default())
        .await
        .result()
    {
        Ok(_) => Err("the call to a model that does not exist was answered".to_owned()),
        Err(error) => {
            println!("  error: {}", variant(error));
            match error {
                ClientError::ModelNotFound | ClientError::ModelNotAccessible => Ok(()),
                other => Err(format!(
                    "expected ModelNotFound or ModelNotAccessible, got {}",
                    variant(other)
                )),
            }
        }
    }
}
