# Changelog

All notable changes to `br-llm-clients-api-openai-chat-completions` are documented here. The git tag
`br-llm-clients-api-openai-chat-completions/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

Built on `br-llm-clients-common-call` 0.2, `br-llm-clients-core` 0.5 and `br-llm-messages` 2.1.
The `Dialect` it implements is the one of common-call 0.2, not the one of 0.1, so a provider
library built on 0.1.0 does not compile against it. With the six new options at their defaults,
`encode` gives the same body bytes as 0.1.0 for a history whose rendering holds the same
messages; a test encodes six requests and compares the bytes with bodies made by the tag
`br-llm-clients-api-openai-chat-completions/v0.1.0`. The decoding and the errors are those of
0.1.0, except for the ending, the response id, the four new quota codes, and a `Usage` that is
now given when one of `prompt_tokens` and `completion_tokens` is present, where 0.1.0 needed
both; all are described below.

### Changed

- Breaking: `encode` takes the `RenderedHistory` that `run` of common-call made with
  `render_history` of the core, beside the request, and encodes it. The dialect no longer builds
  the history itself. A `User` message becomes one `tool` message per tool result, in the order
  of the parts, then one `user` message for its text and image parts when it has any. An
  `Assistant` message becomes one `assistant` message; its text parts and the text of its text
  blocks are its content, its tool calls are `tool_calls` with the wire id of the rendering.
- Thinking, thinking that has replay material and redacted thinking are dropped. The replay
  material and the item reference of a text block or a tool call are not sent. This dialect
  chooses no format of replay material, sends none, and never gives `SignatureDelta` or
  `RedactedThinkingData`.
- A message that holds no part after the drops is omitted, and its neighbours of the same role
  are merged into one message before they are written. This follows the Thinking rule of
  "Rendering of a history" in core 0.5: a `Block` part that the client library drops counts
  as absent, merges the neighbours of the same role again and never sends an empty message.
  The thinking of an omitted message is dropped and never moved to another message.
- `InvalidMessages` when the last message that the dialect sends is an `assistant` message,
  when nothing is left to send, and, with a reason that names it, for an image in an
  `Assistant` message or in a tool result, for a block other than thinking, redacted thinking
  or text (for example a provider tool block), and for a kind of part, block or image source
  that this version does not know.
- Breaking: `ProviderEvent::Stop` carries a `ResponseEnding`: `stop` is `finished`, `length` is
  `output_exhaustion`, `tool_calls` is `tool_use`, `content_filter` is `filtering`, any other
  non-empty finish reason, `function_call` included, is `unclassified` with the reason as
  received, and `finished` after a refusal is `refusal`. The finish reason `error` still gives
  one `Error` event and no `Stop`.
- A `Usage` event is given as soon as `prompt_tokens` or `completion_tokens` is present. The
  cache counts come only with `prompt_tokens`, the thinking count only with
  `completion_tokens`. `prompt_tokens_details.cache_write_tokens` is the cache creation count.
- `refine_error` gives `QuotaExceeded`, with any status, for the codes `insufficient_quota`,
  `credit_balance_exhausted`, `organization_spend_limit_exceeded`,
  `project_spend_limit_exceeded` and `organization_usage_limit_exceeded`, and nothing else.
- Re-exports: `Usage` stays; `ModelId` and `StopReason` are gone. Added from common-call
  `DialectHooks` and `Layered`, from the core `RenderedHistory`, and from `br-llm-messages`
  `ModelName` and `ResponseEnding`, so that a provider library can build and wrap the dialect.
  The re-exported `Headers` is the type of `br-llm-clients-common-http` 0.2.
- The extra body refuses the member `max_tokens` when `max_tokens_field` is `MaxTokens`, and
  only then; an extra body that was valid in 0.1.0 is valid with the default options.
- `OptionsError::OmitAboveOff` is given only while `omit_effort_above_off` is off, which is the
  default.
- The derived `Debug` of `ChatCompletionsOptions` shows the six new fields, and `PartialEq`
  compares them.

### Added

- `ProviderEvent::ResponseId` with the `id` of a chunk when it is a non-empty string that
  differs from the last one given.
- `ChatCompletionsOptions::with_max_tokens_field(MaxTokensField)`: `MaxCompletionTokens` (the
  default) writes `max_completion_tokens`, `MaxTokens` writes `max_tokens`; the dialect writes
  that one member and never the other. With `MaxCompletionTokens`, a `max_tokens` member of
  `extra_body` is still added, as every member of `extra_body` is.
- `with_reasoning_carry_back_field(name)`: each `assistant` message of the turn in progress
  gets that member, holding the texts of its thinking blocks that have no replay material,
  joined in order with nothing between them, when that string is not empty. The thinking of an
  omitted message is dropped. The option does not look at the reasoning level.
- `with_tool_message_name(bool)`: when on, each `tool` message holds `name`, the tool name of
  its result.
- `with_omit_effort_above_off(bool)`: when on, the reasoning table may hold `Omit` for a level
  above `Off`; the other checks of the table stay.
- `with_tool_only_content(ToolOnlyContent)`: `Omitted` (the default) or `EmptyText`, which
  writes `content` as the empty string on an `assistant` message that has a tool call and no
  text.
- `with_empty_tool_result_text(text)`: a tool result that holds no text is written with this
  text in place of the empty string.
- `MaxTokensField` and `ToolOnlyContent` are `Debug`, `Clone`, `Copy`, `PartialEq` and
  `#[non_exhaustive]`.
- `OptionsError` cases `BlankCarryBackField`, `ReservedCarryBackField { field }` and
  `BlankEmptyToolResultText`.

### Verified

The eight recordings of 0.1.0 are served again to `run` of common-call 0.2, whole and cut in
two at the start and the end of the body, at each line end and one byte on each side of it,
and inside each multi-byte character, and every cut gives the same outcome; the response id of
each stream is the `provider_response_id` of the reply. Splitting a body at every byte is pinned
by the tests of `br-llm-clients-common-sse` and `br-llm-clients-common-call`; the decoder tests
of this crate cut a payload at every byte, and a small stream is cut at every byte through
`run`. The new options, the new quota codes, `cache_write_tokens`
and the shapes of other hosts (`reasoning` and `reasoning_content` fields, a top-level
`cached_tokens`, `max_tokens`, a nested extra body, a tool fragment with no `type` and no `id`)
are tested from the documented shape only; no recording covers them.

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
