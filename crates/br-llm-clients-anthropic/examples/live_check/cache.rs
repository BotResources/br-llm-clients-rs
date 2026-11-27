use br_llm_clients_anthropic::{AnthropicClient, CacheTtl};
use br_llm_clients_core::{CallOutcome, Client, OutputMode};
use br_llm_messages::Uuid;

use crate::report::{QuietSink, answered};
use crate::thread::{Failure, Thread};

const SMALLEST_PROMPT: u64 = 4096;
const RULES: u32 = 320;
const LABELS: [&str; 8] = [
    "amber", "basalt", "cedar", "delta", "ember", "fjord", "garnet", "harbor",
];
const QUESTION: &str = "On which shelf is folder 0042? Answer with the number only.";

struct Counts {
    input: u64,
    read: u64,
}

pub(crate) async fn one_hour_cache_read(client: &AnthropicClient) -> Result<(), Failure> {
    say!("  cache lifetime: {}", CacheTtl::OneHour.as_str());
    let thread = Thread::new(QUESTION)?;
    let request = thread.request(&archive(), Vec::new(), OutputMode::Plain)?;
    let first = counts(
        "first",
        client.call(&request, &mut QuietSink::default()).await,
    )?;
    if first.input <= SMALLEST_PROMPT {
        return Err("the request is not more than 4096 tokens".into());
    }
    let second = counts(
        "second",
        client.call(&request, &mut QuietSink::default()).await,
    )?;
    if second.read == 0 {
        return Err("the second call read nothing from the cache".into());
    }
    Ok(())
}

fn counts(name: &str, outcome: CallOutcome) -> Result<Counts, Failure> {
    let usage = outcome.report().usage().cloned();
    say!("  {name} call:");
    answered(outcome)?;
    let usage = usage.ok_or_else(|| format!("the {name} call reports no usage"))?;
    Ok(Counts {
        input: usage.input_tokens().unwrap_or(0),
        read: usage.cache_read_input_tokens().unwrap_or(0),
    })
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
