# Changelog

All notable changes to `br-llm-clients-api-openai-chat-completions` are documented here. The git tag
`br-llm-clients-api-openai-chat-completions/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `ChatCompletionsDialect`, the `Dialect` of `br-llm-clients-common-call` for the
  streaming Chat Completions API of OpenAI: `new` from a `ClientSettings` and a
  `ChatCompletionsOptions` gives an `OptionsError` or a dialect that is `Send`
  and `Sync` and holds nothing that changes from one call to the next. It adds
  nothing to the trait. It is sync, does no I/O and knows no endpoint, path or
  credential.
- `ChatCompletionsOptions`: a required `ReasoningTable`, an optional reasoning
  field name (read as thinking text in each delta), an optional cache read field
  name (the source of the cached input tokens when the details hold none) and an
  `extra_body` JSON object added to the top level of every request body.
  `Debug`, `Clone`, `PartialEq`; no serialization. Every check runs in the
  `new` of the dialect.
- `ReasoningTable`, `ReasoningRule` (`Send` with a `ReasoningEffort`, `Omit`,
  `Refuse`) and `ReasoningEffort` (`none`, `minimal`, `low`, `medium`, `high`,
  `xhigh`, `max`, in rising order). Two named tables, `openai_effort` and
  `openai_effort_without_none`, and a hand-built one from a rule per level. A
  refused level, or a level the table has no rule for, gives
  `ReasoningNotSupported` from `encode` on every call.
- `OptionsError` (`#[non_exhaustive]`, hand-written `Display` and `Error`): a
  table that is not non-decreasing, `Off` mapped above none, `Omit` above `Off`,
  none above `Off`, a blank reasoning field, a reasoning field that the decoder
  reserves (`content`, `refusal`, `tool_calls`, `role`, `audio`,
  `function_call`), a blank cache read field, and an `extra_body` member that the
  dialect writes (`model`, `messages`, `stream`, `stream_options`,
  `max_completion_tokens`, `reasoning_effort`, `tools`, `response_format`). Each
  names its level, field or member.
- Encoding. The body holds `model`, `messages`, `stream` (always `true`),
  `stream_options` (`include_usage`), `max_completion_tokens`, `reasoning_effort`
  when the table sends one, `tools`, `response_format` and the `extra_body`
  members, and nothing else; the only header is `Content-Type`. A system prompt
  is the first message. A user message becomes `tool` messages for its results,
  then one `user` message for its text and image blocks (a string for one text,
  parts otherwise; images are `data:` urls without a `detail`). A relay is one
  `user` message of text parts. An assistant message holds its text and
  structured blocks as text parts and its tool calls as `tool_calls`; thinking
  is dropped and a message left empty is omitted. Generated tool call ids are
  sent as they are. A cache requirement writes nothing and is never refused.
  Tools become `function` tools with an object schema (`required` only when a
  parameter is required, no `additionalProperties`, no `strict`); a structured
  output becomes a `json_schema` `response_format` without `strict`. No
  `tool_choice` and no `parallel_tool_calls`. Encoding twice gives equal bytes.
- Refusals before sending, in this order: `ReasoningNotSupported`,
  `InvalidMessages` (the last message is an assistant message, or a block this
  version does not know), `ToolSchemaRejected` (a parameter kind this version
  does not know), `OutputShapeRejected` (a shape name outside 1 to 64 characters
  of `A-Z a-z 0-9 _ -`, or a parameter kind this version does not know).
- Reading the stream. The sentinel `[DONE]` gives `Complete` alone and drops a
  held tool call; the end of the body without it is `MarkerMissing`. Content,
  the optional reasoning field and `refusal` open the blocks `text` (or
  `structured`), `reasoning` and `refusal`; a new block closes the open one.
  Tool call fragments are held by index (0 to 1023) and given after the other
  content at the finish, in increasing index order, with the whole arguments
  text (`{}` when blank, nothing when blank and the finish is `length`). The
  finish reason maps to the stop reason (`stop`, `length`, `tool_calls`,
  `content_filter`, anything else as `Other`; `stop` after a refusal is
  `Refusal`); the finish reason `error` and a top-level `error` give one `Error`
  event. `Model` follows the `model` of the chunks and `Usage` the `usage`
  (input, output, cache read, cache write, thinking tokens as the wire counts
  them). Metadata is ignored; a named field of the wrong type, output the
  library cannot represent (`audio`, `function_call`, a tool call that is not a
  `function`), more than one choice, a second finish reason, output after the
  finish, a tool call with no name at a finish other than `error`, and a number
  that is not a non-negative integer are a `PayloadError` that names the place.
- Errors. `error_code` reads the `code`, else the `type`, of the `error` object
  of a body; `refine_error` gives `QuotaExceeded` for the code
  `insufficient_quota` with any status and nothing else, so the generic table of
  common-call decides the rest. `MaxOutputTokensTooLarge`, `ToolSchemaRejected`
  and `OutputShapeRejected` are never given from a provider answer.
  `request_id` is the first `x-request-id` header, in any case, when it is
  non-empty text.
- Re-exports, so that a provider library needs no other import for the dialect:
  from `br-llm-clients-common-call` `Dialect`, `EncodedRequest`, `End`,
  `ErrorInfo`, `Headers`, `Payload`, `PayloadDecoder`, `PayloadError`,
  `ProviderBlock` and `ProviderEvent`; from `br-llm-clients-core`
  `CacheRequirement`, `ClientError`, `ClientRequest`, `ClientSettings`,
  `MaxOutputTokens`, `OutputMode` and `ReasoningLevel`; from `br-llm-messages`
  `ModelId`, `StopReason` and `Usage`; from `serde_json` `Map` and `Value`, the
  types of the extra body.
- Derives. `ReasoningTable`, `ChatCompletionsOptions` and `OptionsError` are
  `Debug`, `Clone` and `PartialEq`; `ReasoningRule` and `ReasoningEffort` are
  `Debug`, `Clone`, `Copy` and `PartialEq`. `ChatCompletionsDialect` derives
  nothing and is `Send` and `Sync`.

### Verified against the real API

Eight raw responses of api.openai.com, recorded on 2026-10-19, are kept as test data in
`src/recordings` (the bodies byte for byte, with the response headers) and served
unchanged to `run()`: a text reply, a tool call, a reply cut by `length`, a reasoning
model (`gpt-5.4-mini`, effort `low`, `openai_effort` table), a refusal under a
`json_schema` output, a 401 (`text/plain`, with an extra top-level `status` member), a
404 (`model_not_found`) and a 400 (`empty_array`). Each stream is also served cut in two
at every byte boundary, and each cut gives the same events, step, usage, model and
request id as the whole stream. What the recordings show:

- The frames are `data:` lines followed by the sentinel `[DONE]`. Every chunk carries
  `obfuscation`, `service_tier`, `system_fingerprint` (also null) and `usage: null`;
  the decoder ignores them as metadata. The first delta is
  `{"role":"assistant","content":"","refusal":null}`, or `"content":null` with
  `"refusal":""` for a refusal, and gives no event.
- The finish chunk comes first and the usage chunk, with `choices: []`, after it; the
  usage holds `prompt_tokens`, `completion_tokens`, `total_tokens`,
  `prompt_tokens_details.cached_tokens`, `completion_tokens_details.reasoning_tokens`
  and the audio and prediction counts, which are ignored. A reported zero stays
  `Some(0)`; `cache_write_tokens` is absent and stays `None`.
- A tool call streams as one fragment with `index`, `id`, `type` and `name`, then
  fragments with `index` and `arguments` only; the arguments text is kept as received,
  `{"city": "Paris"}` with its space.
- `gpt-5.4-mini` at effort `low` streams no reasoning text on this API: the step holds
  no thinking and the reasoning tokens (11 of 21) are counted in the usage.
- A refusal streams as `refusal` deltas and ends with `stop`; the step is a `Refusal`
  with the refusal text when the output is plain. With the structured output of the
  recording, common-call judges the reply `ContentBlocked` (its rule for a refusal
  without a structured value); the sink has still received the refusal text.
- An error answer is a pretty-printed JSON body with the `error` object (`message`,
  `type`, `param`, `code`) whatever the content type; `error_code` gives its `code`
  (`invalid_api_key`, `model_not_found`, `empty_array`) and `refine_error` gives
  nothing. The 401 is `Authentication` and carries no text; the 404 and the 400 are
  `ProviderRejected` with the status, the code and the whole body. The header
  `x-request-id` is on every answer, errors included, and is in the report.

### Known gaps

The following are built from the documented shape only; no recording covers them:
a non-zero `cached_tokens` and any `cache_write_tokens`; the finish reasons
`content_filter`, `function_call` and `error`; an `error` frame inside a stream; a 429
and the code `insufficient_quota`; an over-large `max_completion_tokens`; a reasoning
text field of another host (`reasoning_field`) and a non-OpenAI host in general; the
effort values `minimal`, `xhigh` and `max` and the answer of a model to a value it does
not accept; the request body, which no recording checks against the API.
