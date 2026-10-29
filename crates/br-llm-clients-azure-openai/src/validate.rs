use br_llm_clients_common_call::Secret;
use br_llm_clients_common_http::ApiKeyAuthorizer;

use crate::build_error::AzureOpenAiBuildError;
use crate::version::AzureOpenAiVersion;

const KEY_HEADER: &str = "api-key";
const CREDENTIAL_REASON: &str = "the key holds a character outside the range from `!` to `~`";
const API_VERSION_REASON: &str =
    "the dated api version is not `YYYY-MM-DD`, with or without the suffix `-preview`";
const PREVIEW_SUFFIX: &str = "-preview";

fn is_visible_ascii(text: &str) -> bool {
    text.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
}

pub(crate) fn key_authorizer(key: &Secret) -> Result<ApiKeyAuthorizer, AzureOpenAiBuildError> {
    if !is_visible_ascii(key.expose()) {
        return Err(AzureOpenAiBuildError::InvalidCredential {
            reason: CREDENTIAL_REASON.to_owned(),
        });
    }
    ApiKeyAuthorizer::new(KEY_HEADER, key.clone()).map_err(|error| {
        AzureOpenAiBuildError::InvalidCredential {
            reason: error.to_string(),
        }
    })
}

fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

fn is_dated_value(text: &str) -> bool {
    is_date(text.strip_suffix(PREVIEW_SUFFIX).unwrap_or(text))
}

pub(crate) fn api_version(
    version: &AzureOpenAiVersion,
) -> Result<Option<&str>, AzureOpenAiBuildError> {
    match version {
        AzureOpenAiVersion::V1 => Ok(None),
        AzureOpenAiVersion::Dated(text) if is_dated_value(text) => Ok(Some(text.as_str())),
        AzureOpenAiVersion::Dated(_) => Err(AzureOpenAiBuildError::InvalidApiVersion {
            reason: API_VERSION_REASON.to_owned(),
        }),
    }
}
