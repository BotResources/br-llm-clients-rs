# Changelog

All notable changes to `br-llm-clients-mock` are documented here. The git tag
`br-llm-clients-mock/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `MockClient`, a `Client` of `br-llm-clients-core` that answers from a script
  the test writes. It makes no network call, reads no clock, starts no thread,
  opens no file, uses no global and depends on no async runtime. It is `Send`
  and `Sync`, usable as a trait object, cloneable (a clone shares script, rules,
  fallback and log) and can make handles with other `ClientSettings`
  (`with_settings`). Built with `MockClient::new` or `MockClient::builder`
  (`MockBuilder`: replies, rules, fallback, log on or off). Replies, rules and
  the fallback can be added, set, replaced or removed after the build.
- `Capabilities` and `CapabilitiesSpec` (`everything()` as the starting point),
  `Accepted` (`Any` or `Only` a set, for reasoning levels and image media
  types) and `CapabilitiesError::InconsistentCapabilities`.
- Strictness: the mock is always strict and refuses with the `ClientError` case
  of a real client library, in the order settings first, then request, using
  `ClientRequest::check_messages` of core for the message rules. A refusal takes
  no entry and no rule, sends no event and reports a zero duration only. A
  `CacheRequirement` or an `OutputMode` unknown to the mock gives
  `ProviderRejected` with the code `mock.unknown_value`.
- `Reply` and its checked builders: success, failure, cut, hang and broken
  stream (`ReplyBuilder`, with report values, a named usage for failure, cut and
  broken stream, `gate_at`, `chunking` and `rename_tool_call_ids`).
  `MockBuildError` (`#[non_exhaustive]`) reports a reply that breaks a rule when
  it is built. `ReplySource` makes a reply a value or a function; `Rule` pairs a
  matcher with a reply, with an optional limit of uses. A success is checked
  against the request (`ContentBlocked`, `ReplyUnusable`, `MalformedResponse`).
  `tool_call_id` makes ids of the form `{prefix}-{seq}` or `{prefix}-{seq}-{i}`.
- `derive_events` and `Chunking`: the pure derivation of the `StreamEvent` list
  of a step, with text, thinking, tool call arguments and structured value cut
  into pieces of K Unicode scalar values. Folding the events with the stream
  draft of `br-llm-messages` gives the step back.
- Script and rules: claim order fixed by the first poll, atomic claims, rules
  tried in the order added as they stood when the call began, limits, fallback.
  An exhausted script gives `ProviderRejected` with the code
  `mock.script_exhausted`; an invalid reply function result gives the code
  `mock.invalid_reply`. The codes are exported as `CODE_SCRIPT_EXHAUSTED`,
  `CODE_INVALID_REPLY` and `CODE_UNKNOWN_VALUE`.
- The call log: `LoggedCall` (sequence number, copies of request and settings,
  `Served`, `CallState`), `StateKind`, `calls`, `unused_entries`, `unused_uses`,
  `wait_for_calls`, `wait_for_state` (`LogDisabled` when the log is off).
- `Gate` (`open`, `release`, `waiting`, `is_open`, `wait_for_waiting`) to decide
  when a reply goes on, and `RecordingSink` (events, take, len, wait for
  events, fold with a model id; `FoldError`).
- A call that meets no gate, no hang and no sink that is not ready is complete
  at its first poll. No lock is held while a sink, a matcher or a reply
  function runs; a panic in user code leaves the mock usable.
