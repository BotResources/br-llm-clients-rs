use br_llm_clients_common_http::Response;
use br_llm_clients_core::ClientError;

use crate::call_spec::CallSpec;
use crate::classify::classify_refusal;
use crate::error_info::ErrorInfo;
use crate::secrets_cache::SecretsCache;

pub(crate) const REFUSAL_BODY_CAP: usize = 65_536;

pub(crate) async fn refuse(
    spec: &CallSpec,
    cache: &SecretsCache,
    mut response: Response,
) -> ClientError {
    let status = response.status();
    let delay = response.retry_after_ms().or_else(|| response.retry_after());
    let info = match response.body_mut().collect(REFUSAL_BODY_CAP).await {
        Ok(mut collected) => {
            let secrets = cache.in_effect(spec);
            if collected.cut {
                secrets.trim_cut(&mut collected.bytes);
            }
            let text = secrets.clean(&String::from_utf8_lossy(&collected.bytes));
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
