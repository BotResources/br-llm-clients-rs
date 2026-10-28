#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod access_token;
mod error;
mod form_body;
mod form_client;
mod held;
mod lead;
mod reply_parse;
mod state;
mod static_token;
mod token_authorizer;
mod token_cache;
mod token_form;
mod token_reply;
mod token_source;

pub use access_token::AccessToken;
pub use error::AuthConfigError;
pub use form_client::FormTokenClient;
pub use static_token::StaticToken;
pub use token_authorizer::TokenAuthorizer;
pub use token_cache::TokenCache;
pub use token_form::TokenForm;
pub use token_reply::TokenReply;
pub use token_source::TokenSource;

#[cfg(test)]
mod form_body_tests;
#[cfg(test)]
mod reply_parse_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod token_cache_tests;
#[cfg(test)]
mod token_form_tests;
