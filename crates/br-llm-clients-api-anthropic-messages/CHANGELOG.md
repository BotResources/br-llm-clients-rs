# Changelog

All notable changes to `br-llm-clients-api-anthropic-messages` are documented here. The git tag
`br-llm-clients-api-anthropic-messages/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

With the default options, `encode` gives the same body bytes and the same headers as 0.1.0,
and the decoder and the errors are unchanged. The line of `br-llm-clients-common-call` that
this crate imports moves from 0.1 to 0.2: the `Dialect` of that line has one more operation,
`answered_model`, with a default, so the dialect of this crate is a `Dialect` of line 0.2 only,
and an importer must use `br-llm-clients-common-call` 0.2 to give it to a `CallSpec`.

### Added

- `MessagesOptions`: what can differ from host to host, as typed values. `new` and `Default`
  give the default options (the model in the body, the version in the header, no extra member).
  `with_model_placement`, `with_version_placement` and `with_extra_body` take the options and give
  the options (`with_extra_body` gives the options or an `OptionsError`). It has `Debug`,
  `Clone` and `PartialEq`; its `Debug` shows the names of the members of the extra body, never a
  value.
- `ModelPlacement` (`Body`, the default, and `Omitted`: no `model` member in the body) and
  `VersionPlacement` (`Header`, the default, and `Body(text)`: the member `anthropic_version`
  holds the text as it is, blank included, and `encode` returns no `anthropic-version` header).
  Both are `#[non_exhaustive]`.
- The extra body: a `serde_json::Map` whose members are added, each with its whole value, to the
  top level of every body. A second `with_extra_body` replaces the object. The ten names that the
  dialect writes or may write (`model`, `max_tokens`, `stream`, `messages`, `system`, `tools`,
  `thinking`, `output_config`, `cache_control`, `anthropic_version`) are refused for every
  placement.
- `OptionsError` with the case `ReservedExtraBodyMember { member }`; its text names the member and shows
  no value. `#[non_exhaustive]`.
- `AnthropicMessages::with_options(&ClientSettings, MessagesOptions)`: infallible; `new` gives the
  same dialect as `with_options` with the default options.
- `Map` and `Value` of `serde_json`, re-exported.

### Changed

- `br-llm-clients-common-call` is pinned by path and version 0.2.0. For its tests only, the crate
  also depends on `br-llm-clients-common-http` 0.2.0.
- The closed list of body fields and the list of fields that are never sent hold with the default
  options only: with an extra body a caller can send a top-level member of the never-sent list, and
  the placements change the `model` member and the version header as described above.
- The dialect does not write `answered_model`: it gives the default of the trait, which is nothing.

## 0.1.0

### Added

- `AnthropicMessages`: the dialect of the Anthropic Messages API for `br-llm-clients-common-call`.
  It implements `Dialect`, is built by `AnthropicMessages::new(&ClientSettings)` (infallible, no
  option), is `Send + Sync`, and keeps the model, reasoning level, cache requirement and maximum
  output tokens of the settings. It is the only public item.
- `encode`: one JSON body with the closed field list `model`, `max_tokens`, `stream` (always
  `true`), `messages`, `system`, `tools`, `thinking` (always present), `output_config` and
  `cache_control`; headers exactly `content-type: application/json` and
  `anthropic-version: 2023-06-01`; never a beta header or a credential; never `tool_choice`,
  sampling fields, `display` or a thinking budget.
- Messages: text, base64 image and tool result blocks (tool results first, `content` absent when
  empty, `is_error` only when true, no tool name); a `Relay` joins the `User` message before it or
  becomes a new `User` message; tool calls keep their ids; structured blocks replay as compact JSON
  text; thinking blocks (with their signature) and redacted thinking blocks (with their data) are
  sent only for the turn in progress, in place.
- Refusals before sending, as `InvalidMessages`: a last assistant message, a message with no
  block that can be sent, tool call arguments that are not a JSON object, a thinking block of the
  turn in progress without a signature, a block kind this version does not know. Also
  `StructuredOutputWithToolsNotSupported`, `ToolSchemaRejected` and `OutputShapeRejected` (unknown
  parameter kind), `ReasoningNotSupported` and `CacheNotSupported` (unknown value).
- Reasoning table: `Off` is `thinking` disabled with no effort; `Low`, `Medium`, `High`, `Extra`
  and `Max` are `thinking` adaptive with `output_config.effort` `low`, `medium`, `high`, `xhigh`
  and `max`. Cache: `Required` sets the top-level `cache_control` to ephemeral; `NotRequired` sets
  nothing.
- Tools and structured output: tool schemas as object schemas without `additionalProperties` or
  `strict`; a structured shape as `output_config.format` of type `json_schema`, closed with
  `additionalProperties: false` on every object, the shape description as the schema description.
- `decoder`: a decoder per call that reads the SSE payloads of the stream: `message_start`
  (model, usage), `content_block_start`, `content_block_delta` and `content_block_stop` for text,
  structured, thinking, redacted thinking and tool call blocks, `message_delta` (stop reason and
  usage), `message_stop`, `ping`, and the error event. Event types that are unknown are ignored;
  anything else that does not fit is a `PayloadError`.
- The stop reason is held until `message_stop`; `end_turn`, `max_tokens`, `stop_sequence`,
  `tool_use` and `refusal` map to their `StopReason`, any other non-empty text to `Other`. A tool
  call that received no argument text is held and, only when the stop reason is `tool_use`, closed
  at `message_stop` with `{}` before `Stop`; for any other stop reason it is left open so that the
  call is judged incomplete. A stream that ends without `message_stop` is always a missing marker.
- Usage: cumulative merge of `message_start` and `message_delta` usage; the input is the sum of
  `input_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`; a `Usage` event is
  given only when the record changed and both input and output have been seen; `thinking_tokens`
  is the `output_tokens_details.thinking_tokens` of the API (given by the real API in the
  `message_delta` event), with the rules of the cache fields: `None` while no payload carried it,
  `Some(0)` when it was present with zero, kept by a later payload that does not carry it; a
  `null` counts as absent; an `output_tokens_details` that is not an object, or a
  `thinking_tokens` that is not a whole number from 0 up, is a `PayloadError`.
- Errors: `error_code` reads `error.type`; `refine_error` maps the spend-limit 429
  (`enforced_spend_limit_reached`), the usage-limit 400 prefixes, `prompt is too long` and the
  three documented thinking phrases first, then the table of codes and statuses (`overloaded_error`,
  `rate_limit_error`, `billing_error`, `request_too_large`, `authentication_error`,
  `permission_error`, status 402 and 413 with no code); it sets no delay; `request_id` reads the
  `request-id` header.
- Redacted thinking: the stream shape was recorded on 2026-10-19 from a real call to the Anthropic
  API (thinking enabled, the documented test string that forces redaction). It has 22 consecutive
  `redacted_thinking` blocks, each with its `data` in the `content_block_start`, no delta and a
  `content_block_stop`, then a text block; the recording is a test fixture and the decoder reads
  it as the shape described above, including the padded JSON of the real stream.
