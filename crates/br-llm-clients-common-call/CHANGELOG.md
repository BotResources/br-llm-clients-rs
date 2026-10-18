# Changelog

All notable changes to `br-llm-clients-common-call` are documented here. The git tag
`br-llm-clients-common-call/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `Dialect`, the trait an api library implements for one wire dialect: `encode`
  (pure, gives an `EncodedRequest` or a `ClientError`), `decoder` (a new boxed
  `PayloadDecoder` for one call, from the `OutputMode`), `error_code`,
  `refine_error` and `request_id` (all pure). `Send` and `Sync`, object safe.
- `EncodedRequest` (body bytes and a list of headers), `ErrorInfo` (status,
  provider code, asked delay, text), `Payload` (event name and data text),
  `PayloadError`, the `PayloadDecoder` trait with its `End` result (`Events` or
  `MarkerMissing`), and `ProviderEvent` with `ProviderBlock`, the vocabulary in
  which a decoder tells the library what the provider said.
- `CallSpec`: the dialect (shared trait object), the `Endpoint`, the
  `Authorizer`, the `HttpClient`, the extra headers of the provider, the
  `Secret` values to remove from texts, and the `ModelId`. `Send` and `Sync`.
  The types of `br-llm-clients-common-http` that appear in these signatures
  (`Authorizer`, `Endpoint`, `Headers`, `HttpClient`, `Secret`) are re-exported.
- `run`, the call: borrows a `CallSpec`, a `ClientRequest` and a `StreamSink`,
  gives a `CallOutcome`; its future is `Send`. It checks the messages with the
  check of the core (`InvalidMessages`, nothing sent), asks the dialect to
  encode (a refusal comes back with its case unchanged, nothing sent), builds
  one request, and sends it once with the authorizer; it never retries.
- Headers. The library always sends `Accept: text/event-stream` and replaces any
  `Accept` of the dialect or of the provider; a header of the provider replaces
  a header of the dialect with the same name (case-insensitive). A header that
  the HTTP library refuses, or a name given twice inside one list, is a defect
  of the dialect or of the provider: `ProviderRejected` with no status and no
  code, a text that names which side it came from and never shows a name or a
  value, and nothing is sent. The check also covers a header that is replaced.
- Refusals. A status outside 200 to 299 is a refusal. The body is collected up
  to 64 KiB, read as text (each invalid byte sequence becomes one U+FFFD), and
  every secret of the `CallSpec` is replaced by `[redacted]`, the longest first;
  `error_code` is given that text. If the body cannot be collected the
  `ErrorInfo` holds the status and the delay, no code and an empty text.
- The Generic table. A failure of `send`: an authorizer with no credential or
  an invalid authorization is `Authentication`; a failure of the authorizer to
  fetch, a connect, TLS or timeout failure, or an interrupted send is
  `Transport`; a protocol failure is `MalformedResponse`. A refusal status: 401
  `Authentication`, 403 `PermissionDenied`, 429 `RateLimited` and 503 or 529
  `Unavailable` (both with the delay of the response), another 400 to 499
  `ProviderRejected`, any other `ProviderFailed`, both with the status, the code
  and the text. `refine_error` is asked first; when it gives `RateLimited` or
  `Unavailable` with no delay and the `ErrorInfo` holds one, the library sets
  it. An error inside a stream with no status in 400 to 599 reads the status as
  absent and is `ProviderFailed`; its code and text are cleaned and cut to 64
  KiB before the dialect sees them. A case that a later version of
  `common-http` or `common-sse` adds is read as `Transport` for a send, as
  `StreamTruncated` for a body, and as `MalformedResponse` for the SSE decoder.
- The outcome. Before it is returned, the secrets are removed from every text of
  the `ClientError` (the texts of `InvalidMessages`, `ReasoningNotSupported`,
  `LimitExceeded`, `ToolSchemaRejected`, `OutputShapeRejected`,
  `MalformedResponse`, and the code and the body of `ProviderRejected` and
  `ProviderFailed`), whoever made the text, the longest secret first, and then
  every such text is cut to 64 KiB at a character boundary. A case added to
  `ClientError` later is returned unchanged. The `CallReport` holds the request
  id found by `request_id`, the usage and the model when there were any, and the
  total duration, measured on the tokio clock from the first poll of `run`.
- A reply with a 2xx status is not read yet: `run` gives `StreamTruncated` for
  it.
