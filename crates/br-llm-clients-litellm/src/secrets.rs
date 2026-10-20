use br_llm_clients_common_http::Secret;

const ABBREVIATION_PREFIX: &str = "sk-...";
const ABBREVIATION_TAIL: usize = 4;

pub(crate) fn guard_values(key: &Secret) -> Vec<Secret> {
    let mut values = vec![key.clone()];
    values.extend(abbreviated(key.expose()));
    values
}

fn abbreviated(key: &str) -> Option<Secret> {
    let mut tail: Vec<char> = key.chars().rev().take(ABBREVIATION_TAIL).collect();
    if tail.len() < ABBREVIATION_TAIL {
        return None;
    }
    tail.reverse();
    let text: String = tail.into_iter().collect();
    Secret::new(format!("{ABBREVIATION_PREFIX}{text}")).ok()
}
