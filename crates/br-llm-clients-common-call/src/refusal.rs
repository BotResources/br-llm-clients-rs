use br_llm_clients_common_http::Response;
use br_llm_clients_core::ClientError;

use crate::call_spec::CallSpec;
use crate::classify::classify_refusal;
use crate::error_info::ErrorInfo;

pub(crate) const REFUSAL_BODY_CAP: usize = 65_536;

pub(crate) async fn refuse(spec: &CallSpec, mut response: Response) -> ClientError {
    let status = response.status();
    let delay = response.retry_after();
    let info = match response.body_mut().collect(REFUSAL_BODY_CAP).await {
        Ok(collected) => {
            let text = spec
                .secrets
                .replace(&String::from_utf8_lossy(&collected.bytes));
            let info = ErrorInfo::new(text.as_str()).with_status(status);
            match spec.dialect.error_code(&text) {
                Some(code) => info.with_code(code),
                None => info,
            }
        }
        Err(_) => ErrorInfo::new("").with_status(status),
    };
    let info = match delay {
        Some(delay) => info.with_retry_after(delay),
        None => info,
    };
    classify_refusal(&*spec.dialect, info)
}
