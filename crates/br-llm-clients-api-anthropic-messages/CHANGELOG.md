# Changelog

All notable changes to `br-llm-clients-api-anthropic-messages` are documented here. The git tag
`br-llm-clients-api-anthropic-messages/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

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
  text; thinking and redacted thinking blocks are sent only for the turn in progress, in place,
  with their signature.
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
  is always `None`.
- Errors: `error_code` reads `error.type`; `refine_error` maps the spend-limit 429
  (`enforced_spend_limit_reached`), the usage-limit 400 prefixes, `prompt is too long` and the
  three documented thinking phrases first, then the table of codes and statuses (`overloaded_error`,
  `rate_limit_error`, `billing_error`, `request_too_large`, `authentication_error`,
  `permission_error`, status 402 and 413 with no code); it sets no delay; `request_id` reads the
  `request-id` header.
- Known gap: the stream shape of `redacted_thinking` is built from the intent, not from a recorded
  call; a fixture recorded from a real call is still to be added.
