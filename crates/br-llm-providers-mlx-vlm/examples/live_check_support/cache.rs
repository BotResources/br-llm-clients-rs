use br_llm_clients_core::CallOutcome;

pub struct Reuse {
    pub prompt: Option<u64>,
    pub cached: Option<u64>,
}

impl Reuse {
    pub fn of(outcome: &CallOutcome) -> Self {
        let usage = outcome.report().usage();
        Self {
            prompt: usage.and_then(|usage| usage.input_tokens()),
            cached: usage.and_then(|usage| usage.cache_read_input_tokens()),
        }
    }

    pub fn reused(&self) -> bool {
        self.cached.is_some_and(|cached| cached > 0)
    }

    pub fn describe(&self) -> String {
        let share = match (self.prompt, self.cached) {
            (Some(prompt), Some(cached)) if prompt > 0 => cached
                .checked_mul(100)
                .and_then(|scaled| scaled.checked_div(prompt))
                .map_or_else(|| "?".to_owned(), |share| format!("{share}%")),
            _ => "?".to_owned(),
        };
        format!(
            "prompt {} tokens, cache read {} tokens ({share})",
            count(self.prompt),
            count(self.cached)
        )
    }
}

fn count(tokens: Option<u64>) -> String {
    tokens.map_or_else(|| "?".to_owned(), |tokens| tokens.to_string())
}
