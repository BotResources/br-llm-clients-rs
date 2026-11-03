use br_llm_clients_core::{Client, OutputMode};
use br_llm_messages::TurnEntry;

use crate::Failure;
use crate::checks::call;
use crate::show;
use crate::thread::Thread;
use crate::tools::{results_entry, weather_tool};

const SYSTEM: &str = "You must call get_weather once for each city, then answer.";
const QUESTION: &str = "What is the weather in Paris and in Tokyo?";
const LABEL: &str = "4 tool call";
const MAX_ROUNDS: usize = 5;
const EXPECTED_CALLS: usize = 2;

pub async fn weather(client: &dyn Client) -> Result<bool, Failure> {
    let mut thread = Thread::new(QUESTION)?;
    let mut executed = 0usize;
    for round in 1..=MAX_ROUNDS {
        let request = thread.request(Some(SYSTEM), vec![weather_tool()?], OutputMode::Plain)?;
        let (outcome, _) = call(client, &format!("{LABEL}, round {round}"), &request).await;
        let Ok(response) = outcome.result() else {
            return Ok(false);
        };
        let Some((count, results)) = results_entry(response)? else {
            println!("[{LABEL}] tool calls executed: {executed}");
            return Ok(executed >= EXPECTED_CALLS && show::has_text(response));
        };
        executed = executed.saturating_add(count);
        thread.push(TurnEntry::ModelResponse(response.clone()));
        thread.push(results);
    }
    println!("[{LABEL}] no final answer after {MAX_ROUNDS} rounds");
    Ok(false)
}
