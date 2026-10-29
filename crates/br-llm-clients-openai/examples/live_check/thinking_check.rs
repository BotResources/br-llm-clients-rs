use br_llm_clients_core::ReasoningLevel;
use br_llm_clients_openai::{OpenAiClient, ReasoningSummary};
use br_llm_messages::{ContentBlock, ModelResponse, PlainBlock};

use crate::Failure;
use crate::dialogue::Dialogue;
use crate::pictures::{GREEN_SQUARE, numbers, picture, words};
use crate::setup::Setup;
use crate::sink::answer_text;
use crate::thread::text_block;

const SYSTEM: &str =
    "You think before you answer. You answer with what is asked for, nothing else.";
const PRIMES: [u64; 4] = [11, 13, 17, 19];
const SHOWN_SUMMARY_CHARACTERS: usize = 160;

#[derive(Default)]
struct Thinking {
    blocks: usize,
    with_summary: usize,
    with_replay: usize,
    first_summary: Option<String>,
}

impl Thinking {
    fn read(&mut self, response: &ModelResponse) {
        for block in response.blocks() {
            if let ContentBlock::Thinking(thinking) = block {
                self.blocks = self.blocks.saturating_add(1);
                if thinking.replay().is_some() {
                    self.with_replay = self.with_replay.saturating_add(1);
                }
                let text = thinking.text().trim();
                if !text.is_empty() {
                    self.with_summary = self.with_summary.saturating_add(1);
                    if self.first_summary.is_none() {
                        self.first_summary = Some(
                            text.chars()
                                .take(SHOWN_SUMMARY_CHARACTERS)
                                .collect::<String>()
                                .replace('\n', " "),
                        );
                    }
                }
            }
        }
    }

    fn show(&self) {
        println!(
            "  thinking blocks: {} (with a summary: {}, with replay material: {})",
            self.blocks, self.with_summary, self.with_replay
        );
        if let Some(summary) = &self.first_summary {
            println!("  first summary: {summary}");
        }
    }
}

async fn step(
    dialogue: &mut Dialogue,
    client: &OpenAiClient,
    thinking: &mut Thinking,
    blocks: Vec<PlainBlock>,
) -> Result<Option<String>, Failure> {
    let Some(response) = dialogue.say(client, SYSTEM, blocks).await? else {
        return Ok(None);
    };
    thinking.read(&response);
    let text = answer_text(&response);
    println!("  answer: {text}");
    Ok(Some(text))
}

pub async fn dialogue(setup: &Setup) -> Result<bool, Failure> {
    let profile = setup
        .profile()
        .clone()
        .with_summary(ReasoningSummary::Detailed);
    let level = setup.thinking_level(&profile);
    println!("  reasoning level: {level:?}");
    let client = setup.client_at(&profile, level)?;
    let mut dialogue = Dialogue::new()?;
    let mut thinking = Thinking::default();
    println!("  turn 1");
    let Some(first) = step(
        &mut dialogue,
        &client,
        &mut thinking,
        vec![text_block(
            "Pick one prime number between 10 and 20. Reply with the number only.",
        )?],
    )
    .await?
    else {
        return Ok(false);
    };
    let Some(prime) = numbers(&first)
        .into_iter()
        .find(|number| PRIMES.contains(number))
    else {
        println!("  the first answer holds no prime between 10 and 20");
        return Ok(false);
    };
    println!("  turn 2");
    let Some(second) = step(
        &mut dialogue,
        &client,
        &mut thinking,
        vec![text_block(
            "Multiply your number by 3. Reply with the result only.",
        )?],
    )
    .await?
    else {
        return Ok(false);
    };
    let tripled = prime.saturating_mul(3);
    let second_right = numbers(&second).contains(&tripled);
    println!("  turn 3");
    let Some(third) = step(
        &mut dialogue,
        &client,
        &mut thinking,
        vec![
            text_block("Here is a picture.")?,
            picture(GREEN_SQUARE)?,
            text_block(
                "Name its colour, then add 1 to your last result. Reply as: colour, number.",
            )?,
        ],
    )
    .await?
    else {
        return Ok(false);
    };
    let third_right = words(&third).iter().any(|word| word == "green")
        && numbers(&third).contains(&tripled.saturating_add(1));
    thinking.show();
    println!(
        "  expected: {prime}, then {tripled}, then green and {}",
        tripled.saturating_add(1)
    );
    let thought = level == ReasoningLevel::Off || thinking.blocks > 0;
    Ok(second_right && third_right && thought)
}
