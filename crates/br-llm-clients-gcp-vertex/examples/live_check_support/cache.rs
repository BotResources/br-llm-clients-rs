use br_llm_clients_core::{CallOutcome, ClientRequest, OutputMode};
use br_llm_clients_gcp_vertex::{CacheTtl, VertexClient};
use br_llm_messages::Uuid;

use crate::checks::call;
use crate::report::variant;
use crate::sink::CountingSink;
use crate::thread::Thread;

const RULES: u32 = 320;
const LABELS: [&str; 8] = [
    "amber", "basalt", "cedar", "delta", "ember", "fjord", "garnet", "harbor",
];
const QUESTION: &str = "On which shelf is folder 0042? Answer with the number only.";

pub async fn one_hour_twice(client: &VertexClient) -> Result<(), String> {
    println!("  cache lifetime: {}", CacheTtl::OneHour.as_str());
    let request = long_request()?;
    accepted(
        "first",
        call(client, &request, &mut CountingSink::default()).await,
    )?;
    accepted(
        "second",
        call(client, &request, &mut CountingSink::default()).await,
    )
}

fn accepted(name: &str, outcome: CallOutcome) -> Result<(), String> {
    let usage = outcome.report().usage().cloned();
    if let Err(error) = outcome.result() {
        return Err(format!("the {name} call failed with {}", variant(error)));
    }
    let count = |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |n| n.to_string());
    match usage {
        Some(usage) => println!(
            "  {name} call: input tokens {}, cache write {}, cache read {}",
            count(usage.input_tokens()),
            count(usage.cache_creation_input_tokens()),
            count(usage.cache_read_input_tokens())
        ),
        None => println!("  {name} call: no usage reported"),
    }
    Ok(())
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
