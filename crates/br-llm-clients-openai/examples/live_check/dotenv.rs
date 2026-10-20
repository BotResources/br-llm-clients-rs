use std::collections::HashMap;
use std::fs;

const REPO_ROOT_ENV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env");

pub struct Settings {
    file_values: HashMap<String, String>,
}

impl Settings {
    pub fn load() -> Self {
        let mut file_values = HashMap::new();
        for path in [".env", REPO_ROOT_ENV] {
            if let Ok(text) = fs::read_to_string(path) {
                for (name, value) in parse(&text).filter(|(_, value)| !value.is_empty()) {
                    file_values.entry(name).or_insert(value);
                }
            }
        }
        Self { file_values }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        std::env::var(name)
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| self.file_values.get(name).cloned())
            .filter(|value| !value.is_empty())
    }
}

fn parse(text: &str) -> impl Iterator<Item = (String, String)> + '_ {
    text.lines().filter_map(parse_line)
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (name, value) = line.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), unquote(value.trim()).to_owned()))
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}
