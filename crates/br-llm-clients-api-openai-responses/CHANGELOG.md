# Changelog

All notable changes to `br-llm-clients-api-openai-responses` are documented here. The git tag
`br-llm-clients-api-openai-responses/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## 0.2.0

Dated 2026-10-29. This is a breaking line: the library implements the `Dialect`
of `br-llm-clients-common-call` 0.2 and names the types of `br-llm-clients-core`
0.5 and of `br-llm-messages` 2.1, so code that compiles against 0.1.0 does not
compile against 0.2.0. A release of 0.1 on its tag stays valid and is not
changed.

### Added

- `Options` has three new public fields, each `None` by default, which sends
  nothing: `prompt_cache_key` (a text, sent as `prompt_cache_key` as it is, with
  no check), `service_tier` (a `ServiceTier`, sent as `service_tier`) and
  `cache_mode` (a `PromptCacheMode`, sent as `prompt_cache_options` with the one
  member `mode`). `ServiceTier` is `Auto`, `Default`, `Flex`, `Priority`,
  `Scale`, `Fast` or `Ultrafast`, sent as `auto`, `default`, `flex`, `priority`,
  `scale`, `fast` and `ultrafast`. `PromptCacheMode` is `Implicit` or
  `Explicit`, sent as `implicit` and `explicit`. Both types are
  `#[non_exhaustive]`, `Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`, and have
  no default.
- `ReasoningSummary` has two new values, `Concise` and `Detailed`, sent as
  `reasoning.summary` `concise` and `detailed`.
- `Options::with_extra_body` takes a `Map<String, Value>` and adds its members to
  the top level of every request body; a second call replaces the object, and
  `Options::extra_body` gives it. A member named `model`, `stream`, `store`,
  `instructions`, `input`, `max_output_tokens`, `tools`, `text`, `reasoning`,
  `include`, `prompt_cache_key`, `service_tier` or `prompt_cache_options` is
  refused with the new `OptionsError::ReservedExtraBodyMember { member }`, which
  names the member and never shows a value. The library re-exports `Map` and
  `Value` of `serde_json`.
- The decoder gives a `ResponseId` event with the `id` of the response of
  `response.created`, when that id is a text that is not empty. An id that is
  not a text is a `PayloadError`.
- A tool result that holds an image is sent with an `output` array: one
  `input_text` part per text and one `input_image` part per image, in the order
  of the result, with a first part `Error: ` when the result is an error. A
  result with texts only is sent as one text, as in 0.1.
- The decoder reads the `code`, `message` and `param` of an `error` event also
  under a nested `error` object (the form that Azure documents), when the member
  is not a text at the top level. A text at the top level wins; an `error`
  member that is not an object is ignored.

### Changed

- Breaking: the dependencies are `br-llm-clients-common-call` 0.2,
  `br-llm-clients-core` 0.5 (tag `br-llm-clients-core/v0.5.0`) and
  `br-llm-messages` 2.1 (tag `br-llm-messages/v2.1.0`), the pins of
  common-call 0.2. The test-only dependencies are `br-llm-clients-common-http`
  0.2 and `br-llm-clients-common-sse` 0.1 on their release tags.
- Breaking: the history is rendered by `render_history` of the core, and
  `encode` gets that `RenderedHistory` and only encodes it. The relay, the turn
  in progress and the generated tool call ids of 0.1 are gone. A `ToolCall` and
  its `ToolResult` carry the wire id of the core as `call_id`. A `Block` part
  that holds a text block is sent as a text, a thinking block follows the table
  of replay below, and a redacted thinking block is not sent. An image in an
  assistant message, a `Block` part with any other block, and a part, a result
  part or an image source that this version does not know are refused with
  `InvalidMessages`.
- An assistant message whose parts all drop out gives no item, and the user
  messages directly before and after it are encoded as one user message: the
  parts of the first, then the parts of the second. A rendering whose last
  message is an assistant message is refused with `InvalidMessages`, also when
  all the parts of that message drop out.
- Breaking: the reasoning envelope (`id` and `encrypted_content` of the done
  `reasoning` item) is the replay material of the thinking block, a text payload
  in the format `br-llm-clients-api-openai-responses/reasoning`, in place of the
  `Signature` of 0.1. The `SignatureDelta` of the decoder carries that format. A
  thinking block of the reply turn whose material has this format and holds a
  valid envelope goes back as a `reasoning` item; every other thinking block is
  dropped without error, also a block whose material has another format or a
  JSON payload.
- Breaking: the `Stop` event carries a `ResponseEnding`. `response.completed`
  gives `refusal` when a refusal was seen and `finished` otherwise.
  `response.incomplete` gives `output_exhaustion` for `max_output_tokens`,
  `filtering` for `content_filter`, `unclassified` with the reason for any other
  reason text, and `unclassified` with `incomplete` when the details or the
  reason are absent or null. An empty reason text is now a `PayloadError`; 0.1
  gave a stop reason with an empty text.
- Breaking: the `Usage` event is built with the counting of the core: the cache
  counts are `cache_read_input_tokens` (`cached_tokens`) and
  `cache_creation_input_tokens` (`cache_write_tokens`). A `usage` object that
  holds only one of `input_tokens` and `output_tokens` now gives a `Usage` event
  with that count; 0.1 gave none. A cache count is given only with
  `input_tokens`, and a thinking count only with `output_tokens`.
- Breaking: `Options` is no longer `Copy`; it holds a text and a JSON object.
  Code that copies an `Options` after use must call `clone`.
  `Options::default()`, the two 0.1 fields and `OpenAiResponses::new` are as in
  0.1; `new` still takes the `Options` by value and cannot fail, and the refusal
  of a reserved member is made by `with_extra_body` only.
- Breaking: the `Debug` text of `Options`, and of `OpenAiResponses`, which shows
  its `Options`, changes. It shows the typed fields, only whether
  `prompt_cache_key` is set, and only the names of the members of `extra_body`,
  never the cache key text or a value.
- `OpenAiResponses` stays `Clone`, and a clone copies the JSON object. The
  library does not set `answered_model`, so the default of common-call applies.
  `CacheRequirement` still has no effect on the body and is never read.
- The wire of 0.1 is kept otherwise: an `Options` with the 0.1 fields only gives
  the same body as 0.1 for a history whose rendering holds the same messages,
  and every 0.1 stream is decoded as before, except the `error` event with a
  nested `error` object, the endings, the response id and a usage object with
  one count, as stated above.

### Known gaps

- No recording holds a reply to a request with the new options, an `error`
  event in the Azure form, a tool result with an image, or a usage object with
  one count: these are proved by constructed bodies and streams only.
- A part of the rendering, a result part or an image source that this version
  does not know cannot be built in a test, because the types of the core and of
  `br-llm-messages` give no such value today; the refusal arm is in the code and
  is not exercised.

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
