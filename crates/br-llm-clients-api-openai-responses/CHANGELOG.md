# Changelog

All notable changes to `br-llm-clients-api-openai-responses` are documented here. The git tag
`br-llm-clients-api-openai-responses/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

### Added

- `Options` has three new public fields: `prompt_cache_key` (a text, sent as `prompt_cache_key`), `service_tier` (a `ServiceTier`, sent as `service_tier`) and `cache_mode` (a `PromptCacheMode`, sent as `prompt_cache_options` with the one member `mode`). All default to `None`, which sends nothing. The new types `ServiceTier` (`Auto`, `Default`, `Flex`, `Priority`) and `PromptCacheMode` (`Implicit`, `Explicit`) are `#[non_exhaustive]`.
- `Options::with_extra_body` takes a `Map<String, Value>` and adds its members to the top level of every request body, after the members the dialect writes; `Options::extra_body` gives the object. A member named `model`, `stream`, `store`, `instructions`, `input`, `max_output_tokens`, `tools`, `text`, `reasoning`, `include`, `prompt_cache_key`, `service_tier` or `prompt_cache_options` is refused with the new `OptionsError::ReservedExtraBodyMember`, which names the member and never shows a value. The library re-exports `Map` and `Value` of `serde_json`.
- The decoder reads the `code`, `message` and `param` of an `error` event also under a nested `error` object (the Azure form), when the member is not a text at the top level. A text at the top level wins; an `error` member that is not an object is ignored.

### Changed

- Breaking: `Options` is no longer `Copy`; it holds a text and a JSON object. Code that copies an `Options` after use must call `clone`. `Options::default()`, the two 0.1 fields and `OpenAiResponses::new` are as in 0.1.
- Breaking: the `Debug` text of `Options` (and of `OpenAiResponses`, which shows its `Options`) changes. It shows only whether `prompt_cache_key` is set and only the member names of `extra_body`, never the cache key text or a value.
- Breaking: the line of `br-llm-clients-common-call` is 0.2 in place of 0.1. An importer moves to `common-call` 0.2 and to this version together. The test-only dependency `br-llm-clients-common-http` is in its line 0.2.
- The wire of 0.1 is not changed: an `Options` with the 0.1 fields only gives the same body, and every 0.1 stream is decoded as before, except an `error` event with its members under a nested `error` object, which now gives the code and the members instead of null ones. `CacheRequirement` still has no effect on the body and is not read.

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

### Verified against the real API

- Seven answers of `api.openai.com` recorded on 2026-10-19 are kept as data files and read by the tests: a text reply, a reply with a reasoning item and a function call, a reply cut by `max_output_tokens`, a refusal under a `json_schema` output, and the 401, 404 and 400 refusal bodies. The decoder, `run`, a cut of the stream at every byte, the reasoning carry-back and the error functions are proved against them.
- The recorded stream matches the dialect as written: the order of the events, the `refusal` text field of a refusal part, the status `incomplete` of a cut message item, the `incomplete_details.reason` `max_output_tokens`, the function call whose arguments come as deltas and as one done text, and the `usage` object with `cached_tokens`, `cache_write_tokens` and `reasoning_tokens`.
- The events `response.in_progress`, `response.output_text.done`, `response.refusal.done`, `response.content_part.done` and `response.function_call_arguments.done` are real and are ignored without losing content.
- The `encrypted_content` of a `reasoning` item is not the same text in `response.output_item.added` and in `response.output_item.done`. The signature is made from the done item, as the dialect requires, and goes back as the `reasoning` item of the same turn.
- A refusal under a `json_schema` output gives `ContentBlocked`, and the same recorded stream read in a plain call gives a `Step` with the stop reason `Refusal`.
- The refusal bodies are pretty-printed, the 401 body is sent as `text/plain` and holds an extra top-level `status`, and `error.code` is `invalid_api_key`, `model_not_found` and `missing_required_parameter`. `error_code` reads the code before the type, `refine_error` has no row for them, and the generic table gives `Authentication` for the 401 and `ProviderRejected` with the status and the code for the 404 and the 400. The 401 body holds a masked fragment of the key, and `Authentication` carries no text, so nothing of it leaves the library.
- The header `x-request-id` is present on every recorded answer, the successful ones and the refusals.

### Known gaps

- No recording holds a reasoning summary: the reasoning item came with `summary` empty although a summary was asked. The events `response.reasoning_summary_text.delta` and `response.reasoning_text.delta`, the two line feeds between summary parts and the thinking text taken from a done item are proved by constructed streams only.
- The recorded usage holds only zero cache counts and one non-zero `reasoning_tokens`: a non-zero `cached_tokens` or `cache_write_tokens` is proved by constructed streams only.
- No recording holds a `response.failed` event, an `error` event inside the stream, a 429, an overload or a quota refusal: those cases, and the codes of `refine_error`, are proved by constructed streams and bodies only.
- No recording holds a structured reply, a `response.incomplete` for a content filter, or a function call whose arguments are empty.
