# Changelog

All notable changes to `br-llm-clients-core` are documented here. The git tag
`br-llm-clients-core/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

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
