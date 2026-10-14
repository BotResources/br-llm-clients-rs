# Changelog

All notable changes to `br-llm-clients-mock` are documented here. The git tag
`br-llm-clients-mock/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

### Changed

- Breaking: follows `br-llm-clients-core` 0.2.0 and `br-llm-messages` 0.3.0 (tag
  `br-llm-messages/v0.3.0`). The types of both libraries in the public API are
  now those of these releases; an importer must move to them too, because the
  types of the earlier releases are not the same types. No API of this crate is
  added, removed or renamed.

## 0.1.0

### Added

- `MockClient`, a `Client` of `br-llm-clients-core` that answers from a script
  the test writes. It makes no network call, reads no clock, starts no thread,
  opens no file, uses no global and depends on no async runtime. It is `Send`
  and `Sync`, usable as a trait object, cloneable (a clone shares script, rules,
  fallback and log) and can make handles with other `ClientSettings`
  (`with_settings`). Built with `MockClient::new` or `MockClient::builder`
  (`MockBuilder`: `reply`, `rule`, `fallback`, `log` on or off, `build`). After
  the build, `push_reply`, `push_rule`, `set_fallback` and `remove_fallback`
  change the script.
- `Capabilities` is made from a `CapabilitiesSpec` with `Capabilities::new`,
  which refuses an inconsistent spec with
  `CapabilitiesError::InconsistentCapabilities` (images on, no accepted image
  media type). `CapabilitiesSpec::everything()` is the single starting point
  that accepts any reasoning level and image media type, the largest output
  tokens and every feature; its public fields can be changed before `new`.
  `Accepted` is `Any` or `Only` a set of values (reasoning levels, image media
  types).
- Strictness: the mock is always strict and refuses with the `ClientError` case
  of a real client library, in the order settings first, then request, using
  `ClientRequest::check_messages` of core for the message rules. A refusal takes
  no entry and no rule, sends no event and reports a zero duration only. A
  `CacheRequirement` or an `OutputMode` unknown to the mock gives
  `ProviderRejected` with the code `mock.unknown_value`.
- `Reply` and its checked builders `Reply::success`, `failure`, `cut`, `hang`
  and `broken_stream`, each giving a `ReplyBuilder<K>`. The kind `K` is one of
  the marker types `SuccessKind`, `FailureKind`, `CutKind`, `HangKind` and
  `BrokenStreamKind`, bound by the sealed traits `ReplyKind`, `Reported` and
  `NamesUsage`. This typestate is a deliberate compile-time guarantee: a hang
  has no report values (`model`, `request_id`, `time_to_first_event`,
  `total_duration` need `Reported`), and only a failure, a cut and a broken
  stream can name a `usage` (`NamesUsage`) because a success takes the usage of
  its step. `gate_at`, `chunking` and `rename_tool_call_ids` are on every kind;
  `chunking` and the renaming need a reply that has a step.
  `build` returns a `Reply` or a `MockBuildError`.
- `MockBuildError` (`#[non_exhaustive]`) reports a reply or a rule that breaks a
  rule when it is built: `UsageBreaksCounting` (the cache read and cache write
  tokens together above the input tokens, compared without overflow, or the
  thinking tokens above the output tokens), `EventCountOutOfRange`,
  `GatePositionOutOfRange`, `ChunkSizeZero`, `StepSettingWithoutStep`,
  `FirstEventAfterEnd`, `FirstEventWithoutEvent` and `RuleLimitZero`.
- `ReplySource` makes a reply a value (`ReplySource::value`, or `From<Reply>`)
  or a function (`ReplySource::function`) of one `ReplyContext` (`request`,
  `settings`, `sequence`, `claimed_before`). `Rule` pairs a matcher, a function
  of one `MatchContext` (`request`, `settings`, `sequence`), with a reply
  source, and has an optional limit of uses (`with_limit`). A success is checked
  against the request (`ContentBlocked`, `ReplyUnusable`, `MalformedResponse`).
- `tool_call_id` makes ids of the form `{prefix}-{seq}` or `{prefix}-{seq}-{i}`.
- `derive_events` and `Chunking` (`none`, `text`, `thinking`, `tool_arguments`,
  `structured`): the pure derivation of the `StreamEvent` list of a step, with
  text, thinking, tool call arguments and structured value cut into pieces of K
  Unicode scalar values. `DerivedEvents` gives the list (`events`) and the
  number of events before the finish event (`before_finish`). Folding the events
  with the stream draft of `br-llm-messages` gives the step back.
- Script and rules: claim order fixed by the first poll, atomic claims, rules
  tried in the order added as they stood when the call began, limits, fallback.
  An exhausted script gives `ProviderRejected` with the code
  `mock.script_exhausted`; a reply function that returns a `MockBuildError`
  gives the code `mock.invalid_reply`. The codes are exported as
  `CODE_SCRIPT_EXHAUSTED`, `CODE_INVALID_REPLY` and `CODE_UNKNOWN_VALUE`. A
  failure of the mock to assemble a reply that was already checked gives
  `MalformedResponse`, never a `mock.` code.
- The call log: `LoggedCall` (`sequence`, copies of `request` and `settings`,
  `served`, `state`), `Served` (`Entry`, `Rule`, `Fallback`, `Exhausted`,
  `Refusal`, `Nothing`, `NotYetClaimed`), `CallState` (with `kind`) and
  `StateKind`, `log`, `calls`, `unused_entries`, `unused_uses`,
  `wait_for_calls` and `wait_for_state`. `LogDisabled` is the error of
  `wait_for_state` when the log is switched off.
- `Gate` (`new`, `open`, `release`, `waiting`, `is_open`, `wait_for_waiting`) to
  decide when a reply goes on, and `RecordingSink` (`new`, `events`, `take`,
  `len`, `is_empty`, `wait_for_events`, `fold` with a model id). `fold` returns
  a `FoldError` (`reason`) when the events do not fold into a step.
- The wait futures (`WaitForCalls`, `WaitForState`, `WaitForWaiting`,
  `WaitForEvents`) are `Send`, `Sync`, `Unpin` and `'static`, and `#[must_use]`
  like `MockBuilder`, `ReplyBuilder` and `Chunking`. A waker is released when it
  is replaced or when its future is dropped, and is never cloned, dropped or
  woken under a lock. A waker that panics on wake does not stop the others:
  every waker is woken first, then the first panic is re-raised from `open`,
  `release`, a poll, and also from a drop (the end of a call, a gate pass)
  unless the thread is already panicking, in which case the panic is dropped so
  that the process does not abort. A call that is begun when a waker panics is
  logged as dropped once its future is dropped, never as running forever.
- A call that meets no gate, no hang and no sink that is not ready is complete
  at its first poll. No lock is held while a sink, a matcher or a reply
  function runs; a panic in user code leaves the mock usable.
