use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn candidates() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(".env")];
    paths.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env"));
    paths
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
    if name.contains('\0') || value.contains('\0') {
        return None;
    }
    let value = value.trim();
    let unquoted = ['"', '\'']
        .iter()
        .find_map(|quote| value.strip_prefix(*quote)?.strip_suffix(*quote))
        .unwrap_or(value);
    Some((name.to_owned(), unquoted.to_owned()))
}

fn is_filled(name: &str) -> bool {
    env::var_os(name).is_some_and(|value| !value.is_empty())
}

pub unsafe fn load() {
    for path in candidates() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (name, value) in text.lines().filter_map(parse_line) {
            if !value.is_empty() && !is_filled(&name) {
                // SAFETY: the caller guarantees that no other thread reads or writes the environment.
                unsafe { env::set_var(&name, &value) };
            }
        }
    }
}
