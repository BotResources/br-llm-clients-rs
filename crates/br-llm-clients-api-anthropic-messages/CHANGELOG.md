# Changelog

All notable changes to `br-llm-clients-api-anthropic-messages` are documented here. The git tag
`br-llm-clients-api-anthropic-messages/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

### Added

- `MessagesOptions`, `ModelPlacement`, `VersionPlacement` and `OptionsError` are exported, and
  `Map` and `Value` of `serde_json` are re-exported. `AnthropicMessages::with_options` takes the
  settings and a `MessagesOptions`; for the default options it gives the same dialect as `new`.
  `new` still takes no option.
- `ModelPlacement::Omitted` leaves the member `model` out of the body. `VersionPlacement::Body`
  writes the member `anthropic_version` with its text and drops the header `anthropic-version`.
  `with_extra_body` adds members to the top level of every body and refuses, with
  `OptionsError::ReservedExtraBodyMember`, the ten members that the dialect writes or may write.
  The `Debug` of the options and the text of the error never show a value of the extra body.
- `OptionsError` is the only error type of the crate; 0.1 defined none.
- The decoder gives a `ResponseId` event when `message.id` of `message_start` is a non-empty
  string.

### Changed

- The `Dialect` implemented is the one of `br-llm-clients-common-call` 0.2. `encode` takes the
  `RenderedHistory` that `run` made and encodes it in its order; the crate renders nothing. The
  rendering replaces the 0.1 rules on relayed texts, on the order of the tool results, on the
  structured block and on the turn in progress. A tool result holds image blocks as well as text
  blocks.
- The signature of a thinking block and the data of a redacted thinking block are replay
  material, in the formats `br-llm-clients-api-anthropic-messages/signature` and
  `br-llm-clients-api-anthropic-messages/redacted-data`. `SignatureDelta` and
  `RedactedThinkingData` carry their format. A thinking or redacted thinking block of the turn in
  progress whose replay material is missing or in another format is refused with
  `InvalidMessages`.
- The decoder gives a `ResponseEnding` where 0.1 gave a stop reason: `end_turn` is `finished`,
  `max_tokens` is `output_exhaustion`, `stop_sequence` is `stop_sequence` with no matched
  sequence, `tool_use` is `tool_use`, `refusal` is `refusal`, and any other non-empty string
  (`pause_turn` and `model_context_window_exceeded` included) is `unclassified` with the string.
- Usage: the cache fields of the `Usage` are `cache_read_input_tokens` and
  `cache_creation_input_tokens`. A `Usage` event is given when the payload changed the record
  and the record holds an input count or an output count; a cache count comes only with an input
  count and a thinking count only with an output count.
- With the default options, the headers, the body bytes for a rendering that holds the same
  messages, the order of the headers and of the arrays, and the messages of the errors are those
  of 0.1. The closed field list and the never-sent list hold with the default options only.

### Removed

- The 0.1 refusals of a message with no block and of tool call arguments that are not a JSON
  object: the rendering holds no empty message and the arguments are always a JSON object.

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
