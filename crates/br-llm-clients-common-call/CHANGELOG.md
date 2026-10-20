# Changelog

All notable changes to `br-llm-clients-common-call` are documented here. The git tag
`br-llm-clients-common-call/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

No type, method or text of 0.1.0 is removed or renamed, but the line of
`br-llm-clients-common-http` moves from 0.1 to 0.2, and so do the types that
this crate re-exports. A dialect that compiles against 0.1.0 compiles against
0.2.0 once it uses the types of `common-http` 0.2.

### Added

- `SecretLog`: the secrets that are made while a call runs, such as a token that
  an authorizer fetches inside `send`. `SecretLog::new` (and `Default`) makes an
  empty log; a clone shares the same list; `record` takes a `Secret` by
  reference, keeps a clone, cannot fail, and keeps a secret that equals one it
  holds only once. The log holds at most 1024 secrets and drops the one recorded
  first when a new one comes. It is `Send` and `Sync`, and its `Debug` text is
  its type name only. The library never records into a log and never empties one.
- `CallSpec::with_secret_log` sets the log and replaces an earlier one. A
  `CallSpec` made with `new` holds a log that is empty and that no one else
  holds. The secrets in effect are the list of `with_secrets` and what the log
  holds at the moment a text is cleaned, so a secret recorded while `send` runs
  is removed from the texts of the outcome as the secrets of the list.
- `Dialect::answered_model`: takes the response headers and gives the model that
  the provider says answered, as text, or nothing. It has a default body that
  gives nothing, so a dialect that does not implement it behaves as in 0.1.0.
  When it gives a text that is a valid `ModelId` and in which no secret in
  effect appears, that model stands as the model of the `Step` of a reply with no
  `Model` event, and as the model of the `CallReport` of a reply and of a
  refusal; a later `Model` event replaces it. Any other text is read as absent
  and never makes a call fail.
- `Layered<H>` and `DialectHooks`: a `Dialect` that puts a few changes over an
  inner `Dialect`. `Layered::new(inner, hooks)` takes an `Arc<dyn Dialect>` and
  an `H: DialectHooks`. The order is fixed: `check` of the hooks, then the inner
  `encode`, then `after_encode`, and a `check` error means the inner dialect is
  not asked; `decoder` wraps the decoder of the inner dialect with
  `wrap_decoder`; `refine_error` asks the hooks first and the inner dialect
  second; `error_code`, `request_id` and `answered_model` ask the inner dialect
  first and give its answer to the hooks, whose answer is the answer. Every
  method of `DialectHooks` has a default body that changes nothing, and
  `Layered` adds no refusal, no table and no header of its own.

### Changed

- The line of `br-llm-clients-common-http` is 0.2, because `Secret::appears_in`
  and `Response::retry_after_ms` are new in that line. `Secret`, `Endpoint`,
  `Headers`, `HttpClient` and `Authorizer`, which this crate re-exports, are
  now the types of `common-http` 0.2 and not the types of `common-http` 0.1:
  cargo keeps two copies and a value of one is not accepted as a value of the
  other. Every library that imports this crate in its line 0.2 and names one of
  these types must import `common-http` in its line 0.2 in the same release.
- A refusal body longer than 64 KiB is trimmed at its end before the secrets are
  replaced: the longest suffix of the collected bytes that equals the start of a
  secret in effect (at least one byte, fewer than the whole secret) is removed.
  In 0.1.0 the body was cut first and the secrets were replaced after, so a
  secret that crossed the cut left its first part in the text of the
  `ClientError` and in the text that `error_code` and `refine_error` read. Such a
  refusal now gives a text shorter by the trimmed bytes, and nothing is trimmed
  when the collection is not cut.
- A secret recorded in the `SecretLog` is removed from every text of the outcome
  before the cut to 64 KiB, as the secrets of the list. In 0.1.0 a secret that an
  authorizer made inside `send` was not known to `run`, and a provider library
  removed it only after the cut. The code and the text of an `Error` event are
  cleaned with the secrets in effect too. A `CallSpec` whose log stays empty
  behaves as in 0.1.0.
- The delay of a refusal is read from `retry-after-ms` first, as
  `Response::retry_after_ms` reads it, and from `Retry-After` when the first
  gives nothing. In 0.1.0 only `Retry-After` was read. The delay of an error
  inside a stream is not changed.
- The request id of the `CallReport` is absent when a secret in effect appears in
  it, at any place in the text, tested when the outcome is ready with
  `Secret::appears_in`. In 0.1.0 it was never tested.

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
  which a decoder tells the library what the provider said. `ProviderEvent`,
  `ProviderBlock` and `End` are `#[non_exhaustive]`: a later minor can add a
  case without breaking a dialect.
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
  `error_code` and `refine_error` are given that text uncut (it can be longer
  than 64 KiB after the replacements; the last cleaning of the error cuts it to
  64 KiB). If the body cannot be collected the
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
- The reply stream. A status of 200 to 299 is a reply; its content type is not
  checked. The chunks of the body go to the SSE decoder of
  `br-llm-clients-common-sse` with its default limit, each frame becomes a
  `Payload` and goes to the payload decoder, and the events of a payload go to
  the sink one by one: the payload decoder gets the next payload only after the
  sink returned.
  The frames that the SSE decoder returned before an error are processed first.
  A body failure is `StreamTruncated` for an interrupted body, an idle timeout
  and a total timeout, and `MalformedResponse` for an unsupported encoding and a
  protocol failure; an error of the SSE decoder is `MalformedResponse`, except
  an input that ends inside a frame, which is `StreamTruncated`; a
  `PayloadError` is `MalformedResponse` with its text.
- The first failure stops the call and the stream is not read further; no event
  after it reaches the sink. An `Error` provider event is classified by
  `refine_error` and then by the Generic table. When a failure is not itself an
  `Error` event, the rest of the same list is scanned up to a `Complete`, and
  the first `Error` found replaces the failure. When the failure is an `Error`
  event, it stands, and a later `Error` of the same list does not replace it.
  The list given by `end` is read like the list of a payload.
- `Complete` ends the call: `end` is not called, the body is not read further,
  the SSE decoder is not finished, no later failure of the body counts, the
  events after it in the same list are dropped and the frames after it are not
  decoded. When the body ends with no `Complete`, the SSE decoder is finished
  (a failure is acted on and `end` is not called), then `end` is called once:
  events are read, a `PayloadError` is `MalformedResponse`, and a missing end
  marker is `StreamTruncated`. A call that is complete with no `Stop` is
  `StreamTruncated`, whatever the stream held.
- Normalization. Blocks reach the sink in the order of their start; the events
  of a block that starts while an earlier one is open wait until every earlier
  block has closed. Indexes are made by the library when the first event of a
  block is sent, consecutive from 0. A text block sends nothing until it holds a
  non-blank character; then its start and its blank deltas go one by one just
  before that delta; a text block that stays blank is dropped with its held
  events. A tool call starts with `ToolCallStart`. These make the reply
  `MalformedResponse` when the provider event arrives: an event or an end for a
  key that is not open, a start for a key that is open, a delta that does not
  fit its block, a block event after `Stop`, a second `Stop`, a model that is
  not a valid `ModelId`, a tool name that is not a valid `ToolName`, a provider
  tool call id that starts with the reserved prefix or is already used in the
  messages of the request or in the reply.
- A tool call with no id (or an empty id) gets an id that starts with `brgen_`
  and is unique among the tool call ids of the request and of the reply;
  `is_generated_id` tells whether an id has that prefix.
- The judgement at the end of the stream is made before anything more is sent:
  it replays the events already sent, the events that closing the open blocks
  would send and the finish event on a fresh `StepDraft` (a `StepDraft` cannot
  be copied), and only if that gives a `Step` are those events sent. When it
  fails, nothing more reaches the sink. JSON of a tool call or of a structured
  value is incomplete when it is empty, when the provider did not close its
  block, or when the parser reports an unexpected end of input (the crate
  depends on `serde_json` for this); incomplete JSON is judged by the stop
  reason of the `Stop` event (`ContentBlocked` for a content filter or a
  refusal, `ReplyUnusable` for `max_tokens`, `MalformedResponse` otherwise), any
  other parse failure is `MalformedResponse`. A reply with no non-blank text, no
  tool call and no structured value is `ContentBlocked` for a content filter or
  a refusal and `ReplyUnusable` otherwise (in the structured mode, a content
  filter or a refusal with no structured block and no tool call is
  `ContentBlocked` whatever text it holds). The stop reason is then set from the
  content (`awaiting_tool_results` with a tool call, `end_turn` when the
  provider said `awaiting_tool_results` with none), the structured mode accepts
  one structured block with no text and no tool call, or, when the request
  declares tools, tool calls with no structured block (text can come beside
  them), and any other reply is `MalformedResponse`. An event that the
  `StepDraft` refuses, or a `Step` that it refuses, is `MalformedResponse`.
- The `Step` and the finish event carry the last `Usage` event; the model of the
  `Step` is the last `Model` event or else the `ModelId` of the `CallSpec`. The
  `CallReport` holds the last `Usage` and the last `Model` on success and on
  failure (absent when there was none), and the time to the first event, which
  runs from the start of the call to just before the first call of the sink and
  is absent when the sink got no event.
