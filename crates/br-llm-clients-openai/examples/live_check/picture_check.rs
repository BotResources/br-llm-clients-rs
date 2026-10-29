use br_llm_clients_openai::OpenAiClient;

use crate::Failure;
use crate::dialogue::Dialogue;
use crate::pictures::{BLUE_SQUARE, QUADRANTS_RED_GREEN_BLUE_YELLOW, RED_SQUARE, picture, words};
use crate::sink::answer_text;
use crate::thread::text_block;

const SYSTEM: &str = "You describe pictures. You answer with the words asked for, nothing else.";

fn position(words: &[String], wanted: &str) -> Option<usize> {
    words.iter().position(|word| word == wanted)
}

pub async fn one_picture(client: &OpenAiClient) -> Result<bool, Failure> {
    let blocks = vec![
        text_block(
            "The picture is cut into four quarters. Name the colour of the top-left quarter, then of the bottom-right quarter. Reply with two words.",
        )?,
        picture(QUADRANTS_RED_GREEN_BLUE_YELLOW)?,
    ];
    let Some(response) = Dialogue::new()?.say(client, SYSTEM, blocks).await? else {
        return Ok(false);
    };
    let text = answer_text(&response);
    println!("  answer: {text}");
    let words = words(&text);
    Ok(position(&words, "red").is_some() && position(&words, "yellow").is_some())
}

pub async fn two_pictures(client: &OpenAiClient) -> Result<bool, Failure> {
    let blocks = vec![
        text_block("Picture A:")?,
        picture(RED_SQUARE)?,
        text_block("Picture B:")?,
        picture(BLUE_SQUARE)?,
        text_block("Name the colour of picture A, then of picture B. Reply with two words.")?,
    ];
    let Some(response) = Dialogue::new()?.say(client, SYSTEM, blocks).await? else {
        return Ok(false);
    };
    let text = answer_text(&response);
    println!("  answer: {text}");
    let words = words(&text);
    Ok(matches!(
        (position(&words, "red"), position(&words, "blue")),
        (Some(red), Some(blue)) if red < blue
    ))
}
