use std::path::PathBuf;

pub fn load() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let candidates = [PathBuf::from(".env"), repo_root.join(".env")];
    let Some(text) = candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
    else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let (name, value) = (name.trim(), unquote(value.trim()));
        if name.is_empty() || value.is_empty() || std::env::var_os(name).is_some() {
            continue;
        }
        // SAFETY: called first in main, before any thread exists.
        unsafe { std::env::set_var(name, value) };
    }
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
