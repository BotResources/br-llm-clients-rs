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
  `ApiFamily` and the `Capabilities`, with `with_key_placement` and
  `with_extra_headers`. The constructors check nothing.
- `ApiFamily`: `ChatCompletions` (the `ChatCompletionsOptions` of
  `br-llm-clients-api-openai-chat-completions`), `Responses` (the `Options` of
  `br-llm-clients-api-openai-responses`) and `Messages` (no option). The routes
  are `/v1/chat/completions`, `/v1/responses` and `/v1/messages` under the path
  of the base.
- `KeyPlacement`: `AuthorizationBearer` (the default), `LiteLlmKeyHeader` (the
  header `x-litellm-api-key`) and `ConfiguredKeyHeader` (the header named by the
  setting `litellm_key_header_name` of the proxy). The two header placements
  send `Bearer ` and the key.
- `LiteLlmBuildError` (`#[non_exhaustive]`) with the cases `InvalidCredential`,
  `InvalidKeyHeader`, `InvalidExtraHeader`, `InvalidEndpoint` and
  `InvalidDialectOptions`, each with a `reason` that never holds the key, a
  header value or a body. `LiteLlmClient::new` returns the first failure, in
  that order.
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
  case. The `reason` gives the position of the header and never a value.
- `InvalidEndpoint`: adding the route to the base gives a result longer than the
  limit of an `Endpoint`. `InvalidDialectOptions`: the chat completions options
  are refused by their library, and the `reason` is its text.
- A base built with `Endpoint::plain_http` is accepted; the key then goes in
  clear text.
- The capabilities are the ones the operator declares: a `Capabilities` of
  `br-llm-clients-core`, checked before every send. The library reads none from
  the proxy and infers none from the name.
- The request id of the report is the response header `x-litellm-call-id`; it
  is dropped when it holds the key or its abbreviated form.
- Errors: the library has no error row of its own. It reads the error object of
  the text of an error, gives the `type` as the code, and reads the status from
  the `code` of the error object for an error inside a stream.
- The key and its abbreviated form `sk-...` and the last four characters are
  removed from every text of the outcome.
- The example `examples/live_check.rs`, a developer tool that is not run in CI:
  three checks against a real proxy (a conversation, a weather tool called
  twice, a structured output).
- Re-exports of the types that the public signatures need: `Endpoint`,
  `Secret`, `HttpClient`, `ChatCompletionsOptions`, `ReasoningTable`,
  `ReasoningRule`, `ReasoningEffort`, `Map`, `Value`, `Options`,
  `ReasoningParameter`, `ReasoningSummary`, `Capabilities`, `CapabilitiesSpec`,
  `Accepted`, `CapabilitiesError` and `ReasoningLevel`.
