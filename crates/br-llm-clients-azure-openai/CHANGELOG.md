# Changelog

All notable changes to `br-llm-clients-azure-openai` are documented here. The
git tag `br-llm-clients-azure-openai/v{version}` releases the crate; this crate
is versioned separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## 0.1.0

Dated 2026-10-29. First release. The library is built on
`br-llm-clients-common-call` 0.2, `br-llm-clients-api-openai-chat-completions`
0.2 and `br-llm-clients-api-openai-responses` 0.2 (all by path),
`br-llm-clients-core` 0.5 (tag `br-llm-clients-core/v0.5.0`) and
`br-llm-clients-common-http` 0.2 (tag `br-llm-clients-common-http/v0.2.0`), the
pins of common-call 0.2.

### Added

- `AzureOpenAiClient`, built by `new` from a `ClientSettings`, an
  `AzureOpenAiConfig` and an `HttpClient`, implements `Client` for a classic
  Azure OpenAI resource. Every call is a `POST` to one route of the resource,
  through the dialect of that route, with the key in the header `api-key`.
  `call` is `run` of common-call, and its outcome is given unchanged;
  `settings` gives the settings. The client is `Send`, `Sync` and `Clone`. Its
  `Debug` shows the deployment (the model of the settings) and the endpoint of
  the route without its query, and no key and no header value. `new` does no
  I/O, builds no `HttpClient`, sets no timeout, and the client keeps nothing
  between calls.
- The routes, added after the endpoint of the resource, segment by segment:
  - v1 form: `/openai/v1/chat/completions` and `/openai/v1/responses`, with no
    `api-version`;
  - dated form: `/openai/deployments/{deployment}/chat/completions` and
    `/openai/responses`, each with the query `api-version` set to the dated
    value.
  A query already in the endpoint of the resource is kept and sent. An endpoint
  whose path ends with the segment `openai` or the segments `openai/v1`, or
  whose query holds a pair named `api-version`, is refused. The model of
  the settings goes, unchanged, in the `model` member of the body on every route
  and form; on the dated Chat Completions route it is also the deployment
  segment of the path.
- `new` checks, in this order, the key, the api version, the endpoint of the
  route and the options of the dialect, and gives the first refusal; nothing is
  built after it.
- The key is the only secret given to common-call: it is cut from every error
  text, and a request id that holds it is dropped. No `SecretLog` is set.
- `AzureOpenAiConfig`, with `Debug` and `Clone`, built by `new` from an
  `Endpoint`, a `Secret` key, an `AzureOpenAiApi` and an `AzureOpenAiVersion`;
  it cannot fail and has no default. Its builders check nothing:
  `with_capabilities` adds a `Capabilities`, and `with_id_source` adds an
  `IdSource` for the block ids of the reply (the default of common-call
  otherwise). Its `Debug` shows the endpoint without its query, the route, the
  form, the capabilities and only whether an id source is set; never the key.
- The `Debug` of `AzureOpenAiApi` shows only the name of the Chat Completions
  route, never its options, because the `Debug` of `ChatCompletionsOptions`
  prints the values of the extra body. For the Responses route it shows the
  `Debug` of the `Options`, which names the extra body members and never their
  values.
- `AzureOpenAiApi`, `#[non_exhaustive]`: `ChatCompletions` holds the
  `ChatCompletionsOptions` of the Chat Completions dialect and `Responses` holds
  the `Options` of the Responses dialect, both unchanged. The library sets no
  option of a dialect and adds no member to the body.
- `AzureOpenAiVersion`, `#[non_exhaustive]`: `V1`, or `Dated` with the text of
  an `api-version`.
- `AzureOpenAiBuildError`, `#[non_exhaustive]` with `Debug`, `Clone`,
  `PartialEq`, `Eq`, `Display` and `Error`, with the cases `InvalidCredential`
  (the key holds a character outside `!` to `~`; the reason is a fixed text;
  an empty key cannot be built, `Secret::new` refuses it), `InvalidApiVersion`
  (a dated value that is not `YYYY-MM-DD`, with or without the suffix
  `-preview`; the reason is a fixed text),
  `InvalidEndpoint` (the endpoint path ends with `openai` or `openai/v1`, or its
  query holds a pair named `api-version` in any letter case, with a fixed
  reason; or the endpoint of the route cannot be built, for example a
  deployment name that is `.` or `..` on the dated Chat Completions route, or a
  path that is too long, and the reason is the text of the HTTP library error,
  without its `invalid endpoint:` prefix) and `InvalidDialectOptions` (the Chat
  Completions dialect refuses its options; the reason is the text of its
  `OptionsError`). Each case holds a text `reason`, which never holds the key
  or a header value.
- When the config has a `Capabilities`, every call checks it against the
  settings and the request before the dialect encodes it, and gives the refusal
  of `br-llm-clients-core`. Without it, no capability is checked.
- Error rows before the table of the dialect: the codes `content_filter` and
  `content_policy_violation`, with any status, are `ContentBlocked`; status 404
  with the code `DeploymentNotFound` is `ModelNotFound`; the code
  `context_length_exceeded`, with any status or none, is `ContextTooLong`
  without a limit; on the Responses route only, the code `no_capacity` in a
  stream error (no status) is `Unavailable`, with no delay (a stream error
  carries none).
- The request id is the value of the header `apim-request-id` when it is valid
  text and not empty, else the request id of the dialect.
- Re-exports of `ChatCompletionsOptions`, `ReasoningTable`, `ReasoningRule`,
  `ReasoningEffort`, `MaxTokensField`, `ToolOnlyContent`, `Map` and `Value` of
  the Chat Completions dialect; of `Options` (as `ResponsesOptions`),
  `OptionsError` (as `ResponsesOptionsError`), `ReasoningParameter`,
  `ReasoningSummary`, `ServiceTier` and `PromptCacheMode` of the Responses
  dialect; and of
  `Accepted`, `Capabilities`, `CapabilitiesError`, `CapabilitiesSpec` and
  `IdSource` of `br-llm-clients-core`.
- The example `live_check`, a developer tool that CI does not run, makes five
  checks on each route against a real resource: a plain answer, streaming text
  deltas, a weather tool call answered with a fake result (it passes when the
  tool ran and the final answer is not empty), the token counts of
  the usage, and a call to a deployment that does not exist, which must give
  `ModelNotFound` (it prints what came back). It reads only the environment
  variables `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY`,
  `AZURE_OPENAI_API_VERSION` (`v1`, or a dated value) and
  `AZURE_OPENAI_CHAT_DEPLOYMENT`, all required, and
  `AZURE_OPENAI_RESPONSES_DEPLOYMENT`, optional: when it is not set, the
  Responses route is skipped and the example says so. It never prints the key.

### Known gaps

- The library has no test of its own: the `live_check` example is compiled, and
  was not run against a real resource for this release.
- The paths, headers and error codes come from the documentation of Microsoft,
  read on 2026-10-29, and are not verified against the real service.
