use br_llm_clients_aws_bedrock::BedrockClient;
use br_llm_clients_core::{CallOutcome, ClientError, ClientRequest, OutputMode};
use br_llm_messages::Uuid;

use crate::checks::{call, require};
use crate::report::variant;
use crate::sink::CountingSink;
use crate::thread::Thread;

const SMALLEST_PROMPT: u64 = 4096;
const RULES: u32 = 320;
const LABELS: [&str; 8] = [
    "amber", "basalt", "cedar", "delta", "ember", "fjord", "garnet", "harbor",
];
const QUESTION: &str = "On which shelf is folder 0042? Answer with the number only.";

struct Counts {
    input: u64,
    written: u64,
    read: u64,
}

pub async fn cache_read(client: &BedrockClient, cached: bool) -> Result<(), String> {
    let request = long_request()?;
    if !cached {
        return refused_before_sending(client, &request).await;
    }
    let first = counts(
        "first",
        call(client, &request, &mut CountingSink::default()).await,
    )?;
    require(
        first.input > SMALLEST_PROMPT,
        "the request is not more than 4096 tokens",
    )?;
    let second = counts(
        "second",
        call(client, &request, &mut CountingSink::default()).await,
    )?;
    require(
        second.read > 0,
        "the second call read nothing from the cache",
    )
}

async fn refused_before_sending(
    client: &BedrockClient,
    request: &ClientRequest,
) -> Result<(), String> {
    let outcome = call(client, request, &mut CountingSink::default()).await;
    let report = outcome.report();
    let answered = report.request_id().is_some()
        || report.model().is_some()
        || report.time_to_first_event().is_some();
    match outcome.result() {
        Ok(_) => Err("the model declares no cache, but the call was answered".to_owned()),
        Err(ClientError::CacheNotSupported) if !answered => {
            println!("  refused before sending: CacheNotSupported");
            Ok(())
        }
        Err(ClientError::CacheNotSupported) => {
            Err("CacheNotSupported came after the service answered".to_owned())
        }
        Err(other) => Err(format!(
            "expected CacheNotSupported before sending, got {}",
            variant(other)
        )),
    }
}

fn counts(name: &str, outcome: CallOutcome) -> Result<Counts, String> {
    let usage = outcome.report().usage().cloned();
    if let Err(error) = outcome.result() {
        return Err(format!("the {name} call failed with {}", variant(error)));
    }
    let usage = usage.ok_or_else(|| format!("the {name} call reports no usage"))?;
    let counts = Counts {
        input: usage.input_tokens().unwrap_or(0),
        written: usage.cache_creation_input_tokens().unwrap_or(0),
        read: usage.cache_read_input_tokens().unwrap_or(0),
    };
    println!(
        "  {name} call: input tokens {}, cache write {}, cache read {}",
        counts.input, counts.written, counts.read
    );
    Ok(counts)
}

fn long_request() -> Result<ClientRequest, String> {
    let mut thread = Thread::new()?;
    thread.say(QUESTION)?;
    thread.request(Some(&archive()), Vec::new(), OutputMode::Plain)
}

fn archive() -> String {
    let mut text = format!(
        "Run {}. You keep the archive below. You answer in one short sentence.\n",
        Uuid::now_v7()
    );
    for (number, label) in (1..=RULES).zip(LABELS.iter().cycle()) {
        text.push_str(&format!(
            "Folder {number:04} is on shelf {} of aisle {}, and its label is {label}.\n",
            number % 17,
            number % 29
        ));
    }
    text
}
