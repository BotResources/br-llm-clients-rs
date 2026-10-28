# Changelog

All notable changes to `br-llm-clients-common-auth` are documented here. The git tag
`br-llm-clients-common-auth/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

Released on 2026-10-28.

### Added

- `AccessToken` (a `Secret` and an optional validity), the `TokenSource` trait (an
  object-safe async `fetch` that gives an `AccessToken` or an `AuthorizerError`, and
  `secrets`, which gives none by default) and `StaticToken`, a source of one fixed
  token with no validity.
- `TokenCache`: keeps the last token of one source and is the only place that
  decides when a token is fetched. The validity counts from the start of the
  fetch and is capped at 24 hours. A token is fresh while its time left is more
  than the skew (300 seconds, or half of a validity shorter than 600 seconds) and
  usable while its time left is more than zero; a token with no validity is not
  kept. One fetch at a time, shared by all clones. A fresh token is given at
  once; a usable token is given at once while a fetch runs; a call with no
  usable token waits and gets the result of the fetch. The leader waits for its
  own fetch and, if it fails, gives the usable token it holds. An error is never
  kept. A fetch is cut after 30 seconds of the tokio clock with `FetchFailed` and
  the text `the token fetch timed out`. A call also waits at most 30 seconds
  from the start of the fetch it waits for, then fails the same way, even when
  the leader is never polled again; the next call leads a new fetch. A result
  that comes at or after 30 seconds from the start of its fetch counts as that
  timeout: nothing is kept, and the leader gets the usable token it holds or
  the same error. A dropped or panicking leader leaves no lock held: a
  waiting call leads the next fetch. No fetch starts in the background.
- `TokenAuthorizer::bearer`, an `Authorizer` that sets `authorization: Bearer
  <token>` from a `TokenCache`, and `with_token_observer`, called once with the
  token each time the authorizer gives one, before it gives the header.
- `TokenForm` (ordered fields; `plain`, `secret`, `client_credentials`,
  `jwt_bearer`, `refresh_token`, `secrets`) and `AuthConfigError`
  (`InvalidFormField`, `DuplicateFormField`; non-exhaustive).
- `FormTokenClient` and `TokenReply`: one form-encoded POST through
  `HttpClient::send` with `NoAuthorizer`. This library sets only the header
  `content-type: application/x-www-form-urlencoded`; `common-http` adds the
  headers it manages. No retry, no redirect. The answer is read up to 64 KiB. A
  2xx answer must be a JSON object with a non-empty `access_token`, an optional
  `token_type` equal to `Bearer` in any case, an optional `expires_in` (an integer
  that `as_u64` reads, or a text of digits, which saturates at the largest `u64`)
  and an optional non-empty `refresh_token`. A 429 or 5xx is `FetchFailed`; any
  other status outside 2xx is `Unavailable`. Error texts are fixed: they name the
  status, and add the `error` member only when it is 1 to 64 characters from `a`
  to `z` and `_`, is not equal to a plain value of the form, holds no secret of
  the form and is no part of one.
- `Debug` and the texts of every type never show a token, a secret or a form
  value.
