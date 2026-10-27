# Changelog

All notable changes to `br-llm-clients-core` are documented here. The git tag
`br-llm-clients-core/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.5.0

### Added

- `render_history(&ClientRequest) -> Result<RenderedHistory, ClientError>`: the
  one rendering of a history for a model. It is synchronous, does no I/O and
  keeps no state. It gives `InvalidMessages` with a reason as text. Roles are
  `User` or `Assistant` only: no entry becomes a system instruction. Entries
  are rendered one by one, consecutive entries of one role are merged into one
  message, and in a `User` message the tool results come first, in the order
  of their calls. The content of another agent (`AgentRequest`, `AgentMail`,
  `AgentOutput`, `PartialAgentOutput`) is rendered inside a frame with an
  escaped author and escaped text. The notices (`RequestAccepted`,
  `QueuePosition`, `TaskStarted`, `TaskCompleted`, `TaskFailed`, and a
  `TurnClosure` by its reason) are fixed English sentences in a `<notice>`
  frame. A tool call of a closed AI turn that no result answers gets a built
  failure result; a call that no result answers in a turn that is not closed
  is refused. A call gets a wire id, and its result carries the same id.
  Thinking, redacted thinking and replay material of a turn other than the
  turn named by `reply_turn` are not rendered.
- `RenderedHistory` (with `messages`), `RenderedMessage` (with `role` and
  `parts`), `RenderedRole` (`User`, `Assistant`), `RenderedPart` (`Text`,
  `Image`, `ToolCall`, `ToolResult`, `Block`) and `RenderedResultPart` (`Text`,
  `Image`: the parts of a tool result). `RenderedPart` and `RenderedResultPart`
  are `#[non_exhaustive]`. `RenderedHistory`, `RenderedMessage`,
  `RenderedPart` and `RenderedResultPart` have `Debug`, `Clone` and
  `PartialEq`, and no `Eq`. None of these types is stored data.
- `IdSource` (one operation, `next_block_id`, which gives a new `BlockId`;
  `Send` and `Sync`; usable as a trait object) and `DefaultIdSource`, a unit
  struct with `Default`, `Clone`, `Copy` and `Debug` that builds each identity
  with `Uuid::now_v7`.
- This crate now owns the `v7` feature of `uuid` (version 1): an importer does
  not need to add it.

### Changed

- Breaking: the types of `br-llm-messages` in the public API are now those of
  `br-llm-messages` 2.1.0 (tag `br-llm-messages/v2.1.0`), its line 2. The
  request holds a `ProjectedThread` in place of the wire messages;
  `ModelResponse` and `ResponseEnding` take the place of `Step` and
  `StopReason`; `Usage`, `ModelName` (was `ModelId`), `ToolName`, `PlainText`
  (was `Text`) and `MediaType` (was `ImageMime`) are those of line 2; so
  `CapabilitiesSpec::image_media_types` and `Capabilities::image_media_types`
  are `Accepted<MediaType>`. An importer that also uses `br-llm-messages` must
  move to 2.1 too; the types of messages 0.5 and of messages 2.1 are not the
  same types.
- Breaking: `ClientRequest::new` takes the system prompt, the history (a
  `ProjectedThread`), the turn and the entry of the reply (a `TurnId` and an
  `EntryId`), the tools and the output mode. New accessors `history`,
  `reply_turn` and `reply_entry`; `messages` is removed. A request is checked
  for a thread with at least one turn and for unique tool names only.
- Breaking: `BuildError::NoMessage` becomes `EmptyThread`.
- Breaking: `CallOutcome::success` takes a `ModelResponse`; `result` and
  `into_parts` give a `ModelResponse` on success. `UsageMismatch` compares the
  usage of the report with the usage of the response.
- Breaking: `CallReport` loses `Eq` (it keeps `PartialEq`), because `Usage` has
  no `Eq`.
- Breaking: `StreamSink::send` takes a `ResponseDelta` by value in place of a
  `StreamEvent`.
- Breaking: `ClientError::ReplyUnusable` holds `ending: ResponseEnding` in
  place of `stop_reason`, and its message prints the ending.
  `ClientError::ImageMediaTypeNotSupported` holds a `MediaType`.
- Breaking: the stored schema is `br-llm-clients-core/2` (`SCHEMA_VERSION`).
  In a stored `CallOutcome` the key `response` (the JSON of a `ModelResponse`)
  replaces the key `step`. A document of the schema `br-llm-clients-core/1` is
  refused, and the refusal names the schema found. This crate holds no
  migration.
- Breaking: loading is stricter. A `ClientSettings` or a `CallOutcome` checks
  its schema before anything else. An unknown key is now refused also in the
  cases without fields (such as `ClientError::Transport`,
  `OutputMode::Plain` and the plain kinds of a parameter). An explicit `null`
  for an optional item is refused (an absent item is omitted). A key given
  twice is refused. A duration with `nanos` of 1000000000 or more is refused.
- The capability check reads the images of the thread. It still does not
  render the history or check it, and never gives `InvalidMessages`. A
  `RuntimeNotice` entry of the thread is ignored by the check.
- Breaking: the `Relay` rules of `ClientRequest::check_messages` and the
  system-role carrier of notices are replaced by the rendering of a history.
  The rules that an adapter must apply on the first message and the last
  message of the rendering stay with the client library.
- `render_history` refuses a `RuntimeNotice` entry with `InvalidMessages`; the
  reason names `runtime_notice`.

### Removed

- `ClientRequest::check_messages` and `ClientRequest::messages`.
- `BuildError::MisplacedRelay`.

### Notes

- `StructuredShape::name` still gives a `&str`; the model of `ClientSettings`
  is a `ModelName`.
- `IdSource` is not data: it is not stored and carries no schema, like
  `Client` and `StreamSink`.
- `DefaultIdSource` has two panic paths of `Uuid::now_v7` (a system clock
  before 1970, a system that gives no random bytes).

## 0.4.1

### Added

- The capabilities contract of the intent of core 0.4 as corrected on
  2026-10-19. The public items, all at the root of the crate, are `Accepted`
  (`Any` or `Only` with a list, no public operation), `CapabilitiesSpec` (a
  struct of eight public fields, with `everything`, `nothing` and
  `max_output_tokens_of`), `Capabilities` (`new`, `normalized`, `everything`,
  `nothing`, `check`, `spec` and the accessors, including
  `accepts_structured_output_with_tools`) and `CapabilitiesError` (one case,
  `InconsistentCapabilities`).
- `Capabilities::check` takes a `ClientSettings` and a `ClientRequest` and runs
  eight steps in a fixed order: reasoning level, cache, output tokens, tools,
  structured output, structured output with tools, image, image media type.
  The first failure is returned, with the `ClientError` case of its step. It
  does not check the messages and never gives `InvalidMessages`.
- The new capability modules deny `clippy::unwrap_used`,
  `clippy::expect_used`, `clippy::indexing_slicing`,
  `clippy::arithmetic_side_effects` and `clippy::panic`; the two modules of the
  check also deny `clippy::wildcard_enum_match_arm`. No other module of the
  crate carries this lint.

### Notes

- Additive: no released item, signature or behaviour changes. The mock 0.4.0
  keeps compiling unchanged. It holds its own `Accepted` and its own
  `CapabilitiesError`, the same as those of this crate, and a
  `CapabilitiesSpec` with the same eight fields. Its `Capabilities` has no
  public `nothing`, `normalized` or `check` and none of the accessors of this
  crate, and its `CapabilitiesSpec` has no `nothing` and no
  `max_output_tokens_of`. The types of this crate are new types, not those of
  the mock.
- `CapabilitiesSpec` and `CapabilitiesError` are not `#[non_exhaustive]`: a new
  field or case is a breaking change of this crate.
- `Accepted`, `CapabilitiesSpec`, `Capabilities` and `CapabilitiesError` are not
  stored data: they are not saved or loaded and carry no schema.

## 0.4.0

### Changed

- Breaking: the types of `br-llm-messages` in the public API (`Step`, `Usage`,
  `StreamEvent`, `WireMessage`, `ModelId`, `ToolName`, `Text`, `StopReason`,
  `ImageMime`) are now those of `br-llm-messages` 0.5.0 (tag
  `br-llm-messages/v0.5.0`), its line 0.5. An importer that also uses
  `br-llm-messages` must move to 0.5 too; the types of 0.4 and of 0.5 are not
  the same types. These types keep the rules they had in 0.4: what 0.5
  changes (a required id on a user input, entry ids unique in a
  conversation, `DuplicateEntryId` in place of `DuplicateTurnId`, the
  conversation schema `br-llm-messages/3`) lies in types this crate does not
  use. The stored schema `br-llm-clients-core/1` and its JSON are unchanged.
- Contract: the intent now states that the types of `br-llm-messages` it
  names are those of its line 0.5. Nothing else in the contract changes.

## 0.3.0

### Added

- `Client::settings(&self) -> &ClientSettings`: the second operation of a
  client, which gives back the `ClientSettings` the client is bound to. It is
  pure (not async, no I/O) and gives the common settings only; the
  configuration of a provider library stays private to that library.

### Changed

- Breaking: `Client` gains the required method `settings`. Every implementor
  must add it. The trait stays object-safe, `Send` and `Sync`.
- Breaking: the types of `br-llm-messages` in the public API (`Step`, `Usage`,
  `StreamEvent`, `WireMessage`, `ModelId`, `ToolName`, `Text`, `StopReason`,
  `ImageMime`) are now those of `br-llm-messages` 0.4.0 (tag
  `br-llm-messages/v0.4.0`). An importer that also uses `br-llm-messages` must
  move to 0.4 too; the types of 0.3 and of 0.4 are not the same types. Its
  rules come with it: a `ToolName` is now 1 to 64 ASCII letters, digits, `_`
  or `-`, the first a letter or `_`, so a `ToolSpec`, a
  `ToolSchemaRejected` error or a `CallOutcome` whose step holds a tool call
  with another name no longer builds or loads. The stored schema
  `br-llm-clients-core/1` is unchanged.
- Contract: every main provider accepts a tool name that `br-llm-messages`
  accepts; a provider that still refuses one gives `ToolSchemaRejected`. A tool
  call of a reply whose name is not a valid `ToolName` makes the reply
  `MalformedResponse`, like any error of `br-llm-messages` met while building
  the `Step`.

### Tests

- A client held as a trait object gives back the settings it is bound to,
  also while a call it made still borrows it.
- A stored `ToolSpec`, `ToolSchemaRejected` error or successful `CallOutcome`
  naming a tool that `br-llm-messages` 0.4 refuses does not load.

## 0.2.0

### Changed

- Breaking: the types of `br-llm-messages` in the public API (`Step`, `Usage`,
  `StreamEvent`, `WireMessage`, `ModelId`, `ToolName`, `Text`) are now those of
  `br-llm-messages` 0.3.0 (tag `br-llm-messages/v0.3.0`). An importer that also
  uses `br-llm-messages` must move to 0.3 too; the types of 0.2 and of 0.3 are
  not the same types. No API of this crate is added, removed or renamed, and the
  stored schema `br-llm-clients-core/1` is unchanged.

## 0.1.0

### Added

- `ReasoningLevel` (`Off`, `Low`, `Medium`, `High`, `Extra`, `Max`; ordered; no
  default), `MaxOutputTokens` (greater than zero, fits in 32 bits) and
  `CacheRequirement` (`Required`, `NotRequired`). The scale and the cache
  requirement can grow.
- The typed tool vocabulary: `ToolSpec`, `Parameters`, `Parameter` and
  `ParameterKind` (string, number, integer, boolean, enum, array, object), with
  blank-name, duplicate-name, enum and nesting checks, and `MAX_NESTING_DEPTH`
  (32). No raw JSON schema.
- `OutputMode` (`Plain`, `Structured`) and `StructuredShape`, which follows the
  same nesting limit.
- `ClientSettings` (model, reasoning level, cache requirement, maximum output
  tokens; every field required) and `ClientRequest` (system prompt, wire
  messages, tools, output mode) with the provider-independent build checks and
  `ClientRequest::check_messages`, the shared `InvalidMessages` rules: the list
  starts with a `User` message, every tool call is answered once, by id, in the
  next `User` message, every tool result matches an earlier tool call, and a
  tool call id appears only once in the list.
- `CallReport` and `CallOutcome` (a report plus a `Step` or a `ClientError`).
  On success the usage of the report equals the usage of the step; on failure
  the report keeps the last usage the provider reported, with no relation to
  any step.
- `ClientError`, the one error type of every client library, with
  `is_retryable`, and `LimitKind`. `BuildError`, the error of building a value
  of this crate: `NoMessage`, `DuplicateToolName`, `DuplicateParameterName`,
  `BlankParameterName`, `EmptyEnum`, `BlankEnumValue`, `DuplicateEnumValue`,
  `BlankShapeName`, `ZeroMaxOutputTokens`, `NestingTooDeep`, `MisplacedRelay`
  and `UsageMismatch`. `ReasoningLevel`, `CacheRequirement`, `ParameterKind`,
  `ClientError`, `LimitKind` and `BuildError` are `#[non_exhaustive]`.
- The `Client` trait (one async operation, usable as a trait object, `Send`
  future), the `StreamSink` trait (async, usable as a trait object, `Send`) and
  `BoxFuture`, the boxed `Send` future type of both.
- Serde for the data types. `SCHEMA_VERSION` (`br-llm-clients-core/1`) is the
  schema carried by `ClientSettings` and `CallOutcome`; another schema is
  refused on load. Loading checks every rule again, also when `Parameters`,
  `Parameter` or `ParameterKind` are loaded on their own. A case, value or kind
  unknown to this version is always refused on load. An unknown field is
  refused on stored structures and on cases that carry fields. It is not
  refused on cases without fields, such as `ClientError::Transport` or
  `OutputMode::Plain`, because serde cannot refuse it there.
