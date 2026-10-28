# Changelog

All notable changes to `br-llm-clients-common-auth` are documented here. The git tag
`br-llm-clients-common-auth/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `AccessToken` (a `Secret` and an optional validity), the `TokenSource` trait (an
  object-safe async `fetch` that gives an `AccessToken` or an `AuthorizerError`, and
  `secrets`, which gives none by default) and `StaticToken`, a source of one fixed
  token with no validity. `Debug` of these types never shows a token.
- `TokenCache`: keeps the last token of one source and is the only place that
  decides when a token is fetched. The validity is counted from the moment the
  fetch started and is capped at 24 hours. A token is fresh while its time left is
  more than the skew (300 seconds, or half of the validity when the validity is
  shorter than 600 seconds) and usable while its time left is more than zero; a
  token with no validity is never kept. One fetch at a time per cache, shared by
  all clones. A call with a fresh token waits for nothing; a call with a usable
  token goes on while a fetch runs; a call with no usable token waits and gets the
  result of the fetch, the token or the error, the same as the leader. The leader
  waits for its own fetch; if the fetch fails and the cache holds a usable token,
  the leader gives that token. The error is never kept. A fetch is limited to 30
  seconds on the tokio monotonic clock and then fails with `FetchFailed` and the
  text `the token fetch timed out`. A dropped or panicking leader leaves no lock
  held: a waiting call leads the next fetch. The cache reads no system clock.
- `TokenAuthorizer::bearer`, an `Authorizer` that sets `authorization: Bearer
  <token>` from a `TokenCache`, and `with_token_observer`, a function called once
  with the token each time the authorizer gives a token, before it gives the
  headers. A cache failure is the failure of the authorizer, unchanged.
- `TokenForm` (ordered fields; `plain`, `secret`, `client_credentials`,
  `jwt_bearer`, `refresh_token`, `secrets`) and `AuthConfigError`
  (`InvalidFormField`, `DuplicateFormField`; non-exhaustive). `Debug` of a form
  shows the names of the fields and never a value.
- `FormTokenClient` and `TokenReply`: one form-encoded POST of a `TokenForm`
  through `HttpClient::send` with `NoAuthorizer`, with only the header
  `Content-Type: application/x-www-form-urlencoded` added by this library; the body
  is read up to 64 KiB. No retry, no redirect. A 2xx answer must be a JSON object
  with a non-empty `access_token`, an optional `token_type` equal to `Bearer`
  without regard to case, an optional integer `expires_in` (a number as `as_u64`
  reads it, or a text of digits; too large saturates) and an optional non-empty
  `refresh_token`; anything else is `FetchFailed`. A status 429 or 5xx is
  `FetchFailed`; any other status outside 200 to 299 is `Unavailable`. The error
  texts are fixed: they name the status, and add the `error` member of the body
  only when it is 1 to 64 characters from `a` to `z` and `_`.
