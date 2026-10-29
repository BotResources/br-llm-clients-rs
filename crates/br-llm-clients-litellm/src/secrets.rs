use br_llm_clients_common_http::Secret;

const ABBREVIATION_PREFIX: &str = "sk-...";
const ABBREVIATION_TAIL: usize = 4;

pub(crate) fn key_secrets(key: &Secret) -> Vec<Secret> {
    let mut secrets = vec![key.clone()];
    secrets.extend(abbreviated(key.expose()));
    secrets
}

fn abbreviated(key: &str) -> Option<Secret> {
    let start = key.len().checked_sub(ABBREVIATION_TAIL)?;
    let tail = key.get(start..)?;
    Secret::new(format!("{ABBREVIATION_PREFIX}{tail}")).ok()
}
