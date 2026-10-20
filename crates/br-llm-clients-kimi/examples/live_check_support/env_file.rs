use std::path::PathBuf;

pub unsafe fn load() {
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
        if name.is_empty()
            || value.is_empty()
            || name.contains('\0')
            || value.contains('\0')
            || std::env::var_os(name).is_some_and(|set| !set.is_empty())
        {
            continue;
        }
        // SAFETY: the caller of `load` guarantees that no other thread reads or writes the environment.
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
