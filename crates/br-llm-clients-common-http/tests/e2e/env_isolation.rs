use std::path::PathBuf;
use std::process::Command;

use br_llm_clients_common_http::{NoAuthorizer, SendError};
use hyper::Response;

use crate::support::canned::serve_response;
use crate::support::h2_server::{TlsServer, service};
use crate::support::pki::Pki;
use crate::support::script_body::ScriptBody;
use crate::support::{client, guarded, post};

const CHILD_MARKER: &str = "BR_E2E_ENV_CHILD";
const CA_FILE_VARIABLE: &str = "SSL_CERT_FILE";

#[tokio::test]
async fn given_proxy_and_certificate_store_variables_then_they_are_ignored() {
    let directory = std::env::temp_dir().join(format!("br-llm-clients-env-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("directory");
    let ca_file: PathBuf = directory.join("ca.pem");
    let dead_proxy = "http://127.0.0.1:1";
    let mut command = Command::new(std::env::current_exe().expect("exe"));
    command
        .args([
            "--exact",
            "env_isolation::child_probe",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CHILD_MARKER, "1")
        .env(CA_FILE_VARIABLE, &ca_file)
        .env("SSL_CERT_DIR", &directory)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy");
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(name, dead_proxy);
    }
    let output = tokio::task::spawn_blocking(move || command.output())
        .await
        .expect("join")
        .expect("child");
    let _ = std::fs::remove_dir_all(&directory);
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{report}");
    assert!(report.contains("child probe ran"), "{report}");
}

#[tokio::test]
async fn child_probe() {
    if std::env::var_os(CHILD_MARKER).is_none() {
        return;
    }
    let plain = serve_response("200 OK", &[], b"direct").await;
    let mut response = guarded(client().send(post(&plain.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .expect("the proxy variables must be ignored");
    assert_eq!(
        response.body_mut().collect(10).await.expect("body").bytes,
        b"direct"
    );
    assert_eq!(plain.accepted(), 1);

    let pki = Pki::trusted();
    let ca_file = std::env::var_os(CA_FILE_VARIABLE).expect("ca file variable");
    std::fs::write(&ca_file, &pki.ca_pem).expect("write ca");
    let secure = TlsServer::start(
        pki.acceptor(&[b"http/1.1"]),
        service(|_| async { Ok(Response::new(ScriptBody::new([]))) }),
    )
    .await;
    let error = guarded(client().send(post(&secure.endpoint("/"), "a"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert!(
        matches!(error, SendError::Tls { .. }),
        "the system store must not be read: {error:?}"
    );
    println!("child probe ran");
}
