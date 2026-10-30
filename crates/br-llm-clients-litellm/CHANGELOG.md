# Changelog

All notable changes to `br-llm-clients-litellm` are documented here. The git tag
`br-llm-clients-litellm/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `LiteLlmClient`, the `Client` of `br-llm-clients-core` for a LiteLLM proxy.
  `LiteLlmClient::new` takes a `ClientSettings`, a `LiteLlmConfig` and an
  `HttpClient`, does no I/O and gives a `LiteLlmBuildError` or a client. It
  checks, in this order, the key, the key placement, the extra headers, the
  endpoint of the route and the options of the dialect, and returns the first
  failure. It is `Send`, `Sync` and `Clone`, holds one `CallSpec` behind an
  `Arc`, does not retry and has a hand-written `Debug` that shows no key, no
  header value and no extra body member.
- `LiteLlmConfig`: the base `Endpoint`, the virtual key as a `Secret`, the
  `ApiFamily` and the `Capabilities`, with `with_key_placement`,
  `with_extra_headers` and `with_id_source`. The `IdSource` is the
  `DefaultIdSource` until the operator sets one; the `Debug` text says whether
  one was set and never shows it. The constructors check nothing.
- `ApiFamily`: `ChatCompletions` (the `ChatCompletionsOptions` of
  `br-llm-clients-api-openai-chat-completions`), `Responses` (the `Options` of
  `br-llm-clients-api-openai-responses`) and `Messages` (the `MessagesOptions`
  of `br-llm-clients-api-anthropic-messages`). The routes are
  `/v1/chat/completions`, `/v1/responses` and `/v1/messages` under the path of
  the base. The dialect of the route is built once, when the client is built.
- `KeyPlacement`: `AuthorizationBearer` (the default), `LiteLlmKeyHeader` (the
  header `x-litellm-api-key`) and `ConfiguredKeyHeader` (the header named by the
  setting `litellm_key_header_name` of the proxy). The two header placements
  send `Bearer ` and the key.
- `LiteLlmBuildError` (`#[non_exhaustive]`) with the cases `InvalidCredential`,
  `InvalidKeyHeader`, `InvalidExtraHeader`, `InvalidEndpoint` and
  `InvalidDialectOptions`, each with a `reason` that never holds the key, a
  header value or a body.
- `InvalidCredential`: the key is refused when it holds a byte outside 0x21 to
  0x7E, or a `"` or a `\`.
- `InvalidKeyHeader`: a `ConfiguredKeyHeader` name is refused when it is not a
  valid header name, when `br-llm-clients-common-http` manages it, or when it is
  `authorization`, `x-litellm-api-key`, `accept` or `content-type`, compared
  without regard to case.
- `InvalidExtraHeader`: the extra headers are checked in list order, and the
  first fault is returned. A fault is a name or a value that
  `br-llm-clients-common-http` refuses, a name given twice, or the name
  `accept`, `authorization` or `x-litellm-api-key`, compared without regard to
  case. The `reason` gives the index of the header, counted from 0, says
  whether its name or its value is at fault, and never shows a value.
- `InvalidEndpoint`: adding the route to the base gives a result longer than the
  limit of an `Endpoint`. `InvalidDialectOptions`: the chat completions options
  are refused by their library, and the `reason` is its text.
- The dialect given to `run` is a `Layered` of the route dialect and private
  hooks. The hooks run the `Capabilities` check before the route dialect
  encodes, give the `type` of the error object as the error code, and give the
  response header `x-litellm-call-id` as the request id.
- Errors: two rows of the proxy come first, on the error object, whatever the
  status and the route: the `type` `budget_exceeded` is `QuotaExceeded`, and a
  `type` that ends with `_model_access_denied` is `ModelNotAccessible`. Then the
  route dialect refines the error. Then, for an error with no HTTP status, the
  status in the `code` of the error object is mapped with the generic table.
- The key and its abbreviated form (`sk-...` and the last four characters) are
  the secrets of the call and are removed from every text of the outcome.
- The example `examples/live_check.rs`, a developer tool that is not run in CI.
  It reads every value from environment variables. With no argument it checks a
  cloud proxy (`LITELLM_API_KEY`, `LITELLM_BASE_URL`, `LITELLM_MODEL`); with the
  argument `mlx` it checks a proxy in front of mlx-vlm and oMLX
  (`LITELLM_MLX_BASE_URL`, `LITELLM_MLX_API_KEY`, `LITELLM_MLX_VLM_MODEL`,
  `LITELLM_OMLX_MODEL`). The checks: a plain answer, the streamed deltas, the
  usage, a weather tool called twice, a structured output with three fields
  (skipped on oMLX) and a wrong key that must give `Authentication`.
- Re-exports of the types needed to build a `LiteLlmConfig`: `Endpoint`,
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
