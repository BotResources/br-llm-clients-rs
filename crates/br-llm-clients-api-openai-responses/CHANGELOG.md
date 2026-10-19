# Changelog

All notable changes to `br-llm-clients-api-openai-responses` are documented here. The git tag
`br-llm-clients-api-openai-responses/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `OpenAiResponses`, built from a `ClientSettings` and an `Options`, implements the `Dialect` trait of `br-llm-clients-common-call` for the OpenAI Responses API with `stream` true. Building it cannot fail. It is `Send` and `Sync` and holds nothing that changes from one call to the next.
- `Options` (`reasoning_summary`, `reasoning_parameter`, with the enums `ReasoningSummary` and `ReasoningParameter`), every field defaulted, all three types `#[non_exhaustive]`.
- Encode: a body with `model`, `stream` true, `store` false, `instructions`, `input`, `max_output_tokens`, `tools`, `text`, `reasoning` and `include`, and the header `Content-Type: application/json`. No `previous_response_id`, `truncation`, `tool_choice` or `parallel_tool_calls`. `CacheRequirement` has no effect on the body.
- Messages become `message`, `function_call`, `function_call_output` and `reasoning` items: user text and image parts, tool results (`Error: ` prefix when `is_error`), relays as user messages, assistant text, structured values and tool calls. A redacted thinking block is not sent. A list that ends with an assistant message and a block kind this version does not know are refused with `InvalidMessages`.
- Reasoning: `ReasoningLevel` maps to the efforts `none`, `low`, `medium`, `high`, `xhigh` and `max`; a level above `Off` also sends `reasoning.summary` (unless the option is `None`) and `include` `reasoning.encrypted_content`. With the parameter omitted, `Off` sends nothing and any other level is refused with `ReasoningNotSupported`.
- Reasoning carry-back: the `id` and `encrypted_content` of a `reasoning` item travel in the `Signature` of the thinking block as a JSON envelope, and a thinking block of the turn in progress with a valid envelope goes back as a `reasoning` item. Every other thinking block is dropped without error.
- Tools and structured output: function tools with a typed JSON schema (`required` always sent, `additionalProperties` false, `strict` true exactly when every parameter at every level is required), and `text.format` json_schema for `Structured`, with the shape name checks `OutputShapeRejected` then `LimitExceeded` of kind other.
- Decoder: reads the events of the stream by their `type`, ignores metadata and unlisted event types, keeps tool calls until the terminal event so that they come last, closes text and thinking blocks at the done event of their item, gives the signature only from the done item, and maps `response.completed`, `response.incomplete`, `response.failed` and `error` to provider events with the stop reasons, usage and error envelope of the dialect. A payload that cannot be read is a `PayloadError` that never quotes the payload. A stream with no terminal event ends with `MarkerMissing`.
- `error_code` reads `error.code` then `error.type` of an HTTP refusal body. `refine_error` maps the quota, rate limit, overload, content policy and request-blaming codes to `ClientError`, from the status and the code only. `request_id` reads the `x-request-id` header.
