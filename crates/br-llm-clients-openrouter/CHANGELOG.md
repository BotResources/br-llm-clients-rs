# Changelog

All notable changes to `br-llm-clients-openrouter` are documented here. The git
tag `br-llm-clients-openrouter/v{version}` releases the crate; this crate is
versioned separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `OpenRouterClient`, the `Client` of `br-llm-clients-core` for OpenRouter.
  `OpenRouterClient::new` takes a `ClientSettings`, an `OpenRouterConfig` and an
  `HttpClient`, does no I/O and gives an `OpenRouterBuildError` or a client. It
  checks, in this order, the key, the default base when no base is set, the
  attribution, the extra headers, the endpoint of the route and the options of
  the dialect, and returns the first failure. It is `Send`, `Sync` and `Clone`,
  holds one `CallSpec` behind an `Arc`, does not retry and has a hand-written
  `Debug` that shows the endpoint of the route, the family and the number of
  headers, and no key and no header value.
- `OpenRouterConfig`: the API key as a `Secret`, the `ApiFamily` and the
  `Capabilities`, with `with_base`, `with_app_referer`, `with_app_title`,
  `with_extra_headers` and `with_id_source`. The base is `DEFAULT_BASE`
  (`https://openrouter.ai/api/v1`) until the operator sets one, for example a
  regional base. The `IdSource` is the `DefaultIdSource` until the operator sets
  one. The constructors check nothing. The `Debug` text shows the base, the
  family, the referer and the title, which are not secret, the number of extra
  headers and whether an `IdSource` was set.
- `ApiFamily`: `ChatCompletions` (the `ChatCompletionsOptions` of
  `br-llm-clients-api-openai-chat-completions`), `Responses` (the `Options` of
  `br-llm-clients-api-openai-responses`) and `Messages` (the `MessagesOptions`
  of `br-llm-clients-api-anthropic-messages`). The routes are
  `chat/completions`, `responses` and `messages` under the path of the base. The
  dialect of the route is built once, when the client is built.
- The key is sent as `Authorization: Bearer`. The app referer is sent as
  `HTTP-Referer` and the app title as `X-OpenRouter-Title`, only when the
  operator sets them.
- `OpenRouterBuildError` (`#[non_exhaustive]`) with the cases
  `InvalidCredential`, `InvalidAttribution`, `InvalidExtraHeader`,
  `InvalidEndpoint` and `InvalidDialectOptions`, each with a `reason` that never
  holds the key, a header value or a body.
- `InvalidCredential`: the key is refused when it holds a byte outside 0x21 to
  0x7E, or a `"` or a `\`.
- `InvalidAttribution`: the referer or the title is refused when it is blank or
  is not a valid header value.
- `InvalidExtraHeader`: the extra headers are checked in list order, and the
  first fault is returned. A fault is a name or a value that
  `br-llm-clients-common-http` refuses, a name given twice, or the name
  `accept`, `authorization`, `content-type`, `anthropic-version`,
  `http-referer`, `x-openrouter-title` or `x-title`, compared without regard to
  case. The `reason` gives the index of the header, counted from 0, says whether
  its name or its value is at fault, and never shows a value.
- `InvalidEndpoint`: the default base cannot be made, or adding the route to the
  base gives a result longer than the limit of an `Endpoint`.
- `InvalidDialectOptions`: the chat completions options are refused by their
  library, and the `reason` is its text.
- The dialect given to `run` is a `Layered` of the route dialect and private
  hooks. The hooks run the `Capabilities` check before the route dialect
  encodes, and give the response header `x-generation-id` as the request id.
- On the chat completions route the hooks read each chunk before the dialect.
  OpenRouter repeats the finish reason in the chunk that holds the usage, and
  the dialect refuses a second finish reason: the hooks remove a finish reason
  that equals the first one. A different second finish reason, and output after
  the finish, are still a malformed reply. An error object inside the choice,
  with no error beside the choices, is split out: the dialect reads the chunk
  without the error and its finish reason, so the output, the usage, the id and
  the model of the chunk are kept, then a chunk that holds only the error, so it
  gives the code and the text of the error instead of a fixed text.
- Error code: the code that the route dialect gives; when it gives none, the
  `error_type` of the error object (in its `metadata`, in the object, or beside
  it), else its `code` when it is a string. The route dialect keeps its own
  code, so its own rows still match.
- Errors: the rows of OpenRouter come first, on the error object, whatever the
  status. The `metadata` is read in the error object, else beside it. A
  `metadata.limit_source` of `openrouter_in_flight_budget` is `RateLimited`, any
  other `limit_source` is `QuotaExceeded`. A `metadata` with `reasons` or
  `flagged_input` (moderation) is `ContentBlocked`. The `error_type`
  `payment_required` is `QuotaExceeded`, `context_length_exceeded` is
  `ContextTooLong`, `content_policy_violation` is `ContentBlocked`,
  `rate_limit_exceeded` is `RateLimited`, `provider_overloaded` and
  `provider_unavailable` are `Unavailable`, `authentication` is `Authentication`
  and `permission_denied` is `PermissionDenied`. A status 402, of the response
  or in the `code` of the error object (a number, or a text of digits), is
  `QuotaExceeded`. Then the route dialect refines the error. Then, for an error
  with no HTTP status, the status in the `code` of the error object is mapped
  with a copy of the generic table of `br-llm-clients-common-call` (which is
  private there), with the `error_type`, else the text `code`, of the object as
  the code. The rows rest on the OpenRouter documentation and are not yet
  checked on a live call. On the responses route, an error inside the stream
  reaches the hooks as the dialect rebuilds it, with no `metadata` and no
  `error_type`: only the 402 row and the status mapping can match it, when its
  text `code` is made only of digits.
- The key is the secret of the call: it is removed from every text of the
  outcome, and a request id that holds it is dropped.
- The example `examples/live_check.rs`, a developer tool that is not run in CI.
  It reads `OPENROUTER_API_KEY` (required), `OPENROUTER_MODEL` (default
  `openai/gpt-4o-mini`), `OPENROUTER_BASE_URL` (must be `https` and end with
  `/api/v1`), `OPENROUTER_APP_REFERER` and `OPENROUTER_APP_TITLE`; a blank
  variable counts as unset. Its first argument picks the route: none or `chat`,
  `responses` or `messages`. On the chat route it reads thinking from
  `reasoning` and sends it back in `reasoning`. The checks: a plain answer, the
  streamed deltas, the usage, a weather tool called twice with fake results, a
  structured output with three fields and a wrong key that must give
  `Authentication`. It never prints the key.
- Re-exports of the types needed to build an `OpenRouterConfig`: `Endpoint`,
  `Secret`, `HttpClient`, `IdSource`, `DefaultIdSource`, `Map`, `Value`,
  `ChatCompletionsOptions`, `ReasoningTable`, `ReasoningRule`,
  `ReasoningEffort`, `MaxTokensField`, `ToolOnlyContent`, `Options`,
  `ReasoningParameter`, `ReasoningSummary`, `PromptCacheMode`, `ServiceTier`,
  `MessagesOptions`, `ModelPlacement`, `VersionPlacement`, `Capabilities`,
  `CapabilitiesSpec`, `Accepted`, `CapabilitiesError` and `ReasoningLevel`; the
  three `OptionsError` types as `ChatCompletionsOptionsError`,
  `ResponsesOptionsError` and `MessagesOptionsError`. The `ClientSettings` of
  the client comes from `br-llm-clients-core`, and the `HttpConfig` that builds
  the `HttpClient` from `br-llm-clients-common-http`.
