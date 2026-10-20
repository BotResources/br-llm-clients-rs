use br_llm_clients_common_http::Secret;

use crate::secret_log::{LOG_CAPACITY, SecretLog};

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

fn held(log: &SecretLog) -> Vec<String> {
    log.snapshot()
        .iter()
        .map(|secret| secret.expose().to_owned())
        .collect()
}

#[test]
fn given_a_secret_recorded_twice_when_read_then_the_log_holds_it_once() {
    let log = SecretLog::new();
    log.record(&secret("tok-1"));
    log.record(&secret("tok-1"));
    log.record(&secret("tok-2"));
    log.record(&secret("tok-1"));
    assert_eq!(held(&log), vec!["tok-1", "tok-2"]);
}

#[test]
fn given_a_full_log_when_one_more_secret_is_recorded_then_the_first_recorded_is_dropped() {
    let log = SecretLog::new();
    for number in 0..LOG_CAPACITY {
        log.record(&secret(&format!("tok-{number}")));
    }
    assert_eq!(held(&log).len(), 1024);
    assert_eq!(held(&log).first().map(String::as_str), Some("tok-0"));
    log.record(&secret("tok-1024"));
    let kept = held(&log);
    assert_eq!(kept.len(), 1024);
    assert_eq!(kept.first().map(String::as_str), Some("tok-1"));
    assert_eq!(kept.last().map(String::as_str), Some("tok-1024"));
}

#[test]
fn given_a_full_log_when_a_known_secret_is_recorded_again_then_nothing_is_dropped() {
    let log = SecretLog::new();
    for number in 0..LOG_CAPACITY {
        log.record(&secret(&format!("tok-{number}")));
    }
    log.record(&secret("tok-0"));
    assert_eq!(held(&log).len(), 1024);
    assert_eq!(held(&log).first().map(String::as_str), Some("tok-0"));
}

#[test]
fn given_a_clone_when_a_secret_is_recorded_through_one_then_the_other_sees_it() {
    let log = SecretLog::new();
    let clone = log.clone();
    clone.record(&secret("tok-1"));
    assert_eq!(held(&log), vec!["tok-1"]);
    assert_eq!(held(&SecretLog::default()), Vec::<String>::new());
}

#[test]
fn given_a_log_with_secrets_when_debug_printed_then_the_text_is_the_type_name_only() {
    let log = SecretLog::new();
    assert_eq!(format!("{log:?}"), "SecretLog");
    log.record(&secret("tok-very-secret"));
    assert_eq!(format!("{log:?}"), "SecretLog");
    assert_eq!(format!("{log:#?}"), "SecretLog");
}

#[test]
fn given_a_log_when_used_from_many_threads_then_it_is_send_sync_and_loses_nothing() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<SecretLog>();
    let log = SecretLog::new();
    let workers: Vec<_> = (0..8)
        .map(|worker| {
            let log = log.clone();
            std::thread::spawn(move || {
                for number in 0..50 {
                    log.record(&secret(&format!("tok-{worker}-{number}")));
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker");
    }
    assert_eq!(held(&log).len(), 400);
}
