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
  already reads (`content`, `refusal`, `tool_calls`, `role`, `audio`,
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
  finish, a tool call with no name and a number that is not a non-negative
  integer are a `PayloadError` that names the place.
- Errors. `error_code` reads the `code`, else the `type`, of the `error` object
  of a body; `refine_error` gives `QuotaExceeded` for the code
  `insufficient_quota` with any status and nothing else, so the generic table of
  common-call decides the rest. `MaxOutputTokensTooLarge`, `ToolSchemaRejected`
  and `OutputShapeRejected` are never given from a provider answer.
  `request_id` is the first `x-request-id` header, in any case, when it is
  non-empty text.
- Re-exports of the types that the public signatures name from
  `br-llm-clients-common-call`, `br-llm-clients-core` and `br-llm-messages`, and
  of `serde_json::{Map, Value}` for the extra body, so that a provider library
  needs no other import for the dialect.
