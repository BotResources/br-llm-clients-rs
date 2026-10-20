use crate::{OpenAiResponses, Options};

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn given_the_dialect_when_checked_then_it_is_send_and_sync() {
    assert_send_sync::<OpenAiResponses>();
    assert_send_sync::<Options>();
}

#[test]
fn given_a_default_options_when_read_then_the_summary_is_auto_and_the_parameter_is_sent() {
    let options = Options::default();
    assert_eq!(options.reasoning_summary, crate::ReasoningSummary::Auto);
    assert_eq!(options.reasoning_parameter, crate::ReasoningParameter::Sent);
}

fn source_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "recordings") {
                source_files(&path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn is_test_file(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name.ends_with("_tests.rs") || name.starts_with("test_")
}

#[test]
fn given_the_files_that_are_not_tests_when_read_then_none_names_the_cache_requirement() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    source_files(&root, &mut files);
    let library: Vec<_> = files.iter().filter(|path| !is_test_file(path)).collect();
    assert!(library.len() > 10);
    for path in library {
        let text = std::fs::read_to_string(path).expect("source");
        assert!(!text.contains("CacheRequirement"), "{path:?}");
        assert!(!text.contains("CacheNotSupported"), "{path:?}");
    }
}
