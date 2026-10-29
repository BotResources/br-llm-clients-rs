# Changelog

All notable changes to `br-llm-clients-openai` are documented here. The git tag
`br-llm-clients-openai/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## 0.1.0

Dated 2026-10-30. First release. The library is built on
`br-llm-clients-common-call` 0.2 (tag `br-llm-clients-common-call/v0.2.0`),
`br-llm-clients-api-openai-responses` 0.2 (tag
`br-llm-clients-api-openai-responses/v0.2.0`), `br-llm-clients-core` 0.5 (tag
`br-llm-clients-core/v0.5.0`) and `br-llm-clients-common-http` 0.2 (tag
`br-llm-clients-common-http/v0.2.0`), the pins of common-call 0.2.

### Added

- `OpenAiClient`, built by `new` from a `ClientSettings`, an `OpenAiConfig` and
  an `HttpClient`, implements `Client` for the first-party OpenAI API. Every
  call is a `POST` to `/v1/responses`, on `https://api.openai.com` by default or
  after the base of the config (the origin, with an optional path prefix,
  before `/v1`), through the dialect
  `br-llm-clients-api-openai-responses`, with `Authorization: Bearer <key>`.
  `call` is `run` of common-call, and its outcome is given unchanged; `settings`
  gives the settings. The client is `Send`, `Sync` and `Clone`. Its `Debug`
  shows the model and the endpoint without its query, and no key and no header
  value. `new` does no I/O, builds no `HttpClient`, sets no timeout, and the
  client keeps nothing between calls.
- `new` checks, in this order, the key, the organization, the project, the
  endpoint and the extra body, and gives the first refusal; nothing is built
  after it. No refusal of `new` depends on the settings or the model.
- The key is the only secret given to common-call: it is cut from every error
  text, and a request id that holds it is dropped. The prompt cache key is not a
  secret. No `SecretLog` is set.
- `OpenAiConfig`, with `Debug` and `Clone`, built by `new` from a `Secret` key
  and a `ModelProfile`; it cannot fail and has no default. Its builders check
  nothing: `with_base` (an `Endpoint`), `with_organization` and `with_project`
  (sent as the headers `OpenAI-Organization` and `OpenAI-Project`, only when
  set), `with_prompt_cache_key`, `with_service_tier`, `with_cache_mode`,
  `with_extra_body` (a JSON object whose members go to the top level of every
  body) and `with_id_source` (an `IdSource` for the block ids of the reply; the
  default of common-call otherwise). Its `Debug` shows the profile, the base,
  the tier, the cache mode and the names of the extra body members, and only
  whether the organization, the project, the prompt cache key and the id
  source are set; never the key, a header value, the cache key or a member
  value.
- `OpenAiBuildError`, `#[non_exhaustive]` with `Debug`, `Clone`, `PartialEq`,
  `Eq`, `Display` and `Error`, with the cases `InvalidCredential` (the key holds
  a character outside `!` to `~`), `InvalidOrganization` and `InvalidProject`
  (the value is empty or holds such a character), `InvalidEndpoint` (the
  default base cannot be built or the joined path is too long; the reason is
  the text of the HTTP library error, without its `invalid endpoint:` prefix)
  and `InvalidExtraBody` (the extra body holds a member the dialect reserves;
  the reason is the text of the dialect error, which names the member and never
  a value). Each case holds a text `reason`, which never holds the key, a
  header value or a member value.
- `ModelProfile` (with `Debug`, `Clone` and `PartialEq`; a `Capabilities`, a
  `ReasoningParameter` and a `ReasoningSummary`, read by `capabilities`,
  `reasoning_parameter` and `reasoning_summary`, and `with_summary` to change
  the summary) and the presets `gpt_6_astra` and `gpt_6_1_sol` (levels `Low` to
  `Max`), `gpt_6_1_terra`, `gpt_6_sol`, `gpt_6_luna` and `gpt_5_6_sol` (levels
  `Off` to `Max`), `gpt_5_5`, `gpt_5_4` and `gpt_5_4_mini` (levels `Off` to
  `Extra`), all with an output ceiling of 128000 tokens, caching and the
  reasoning parameter sent, and `gpt_4_1_mini` (level `Off` only, ceiling 32768,
  caching, reasoning parameter omitted). Every preset has the summary `None`.
- The dialect options come from the profile and the config: the reasoning
  parameter and the summary of the profile, the summary forced to `None` when
  the parameter is omitted, and the prompt cache key, the service tier, the
  cache mode and the extra body of the config, none of which is sent when it is
  not set.
- Every call checks the `Capabilities` of the profile against the settings and
  the request before the dialect encodes it, and gives the refusal of
  `br-llm-clients-core`; this library adds no refusal of its own.
- Two error rows before the table of the dialect: status 404 with the code
  `model_not_found` is `ModelNotFound`, and the code `context_length_exceeded`,
  with any status or none, is `ContextTooLong` without a limit.
- Re-exports of `ReasoningParameter`, `ReasoningSummary`, `ServiceTier`,
  `PromptCacheMode`, `Map` and `Value` of the dialect, and of `Accepted`,
  `Capabilities`, `CapabilitiesError`, `CapabilitiesSpec` and `IdSource` of
  `br-llm-clients-core`.
- The example `live_check`, a developer tool that CI does not run, makes five
  checks against the real service: a plain answer, streaming text deltas, a
  weather tool call answered with a fake result, the token counts of the usage,
  and a call to an unknown model that must give `ModelNotFound`. It reads only
  the environment variables `OPENAI_API_KEY` (required), `OPENAI_BASE_URL`
  (optional; unset or empty uses the default host of the library) and
  `OPENAI_MODEL` (optional, default `gpt-5.4-mini`; an id with no preset is
  refused), and never prints the key or a header value. A base is the origin,
  with an optional path prefix, before `/v1`: its scheme is `https` or `http` in
  any letter case, `http` only for `localhost`, `127.0.0.1` or `[::1]`, and a
  base with a query or whose last path segment is `v1` is refused.

### Known gaps

- The library has no test of its own: the `live_check` example is compiled, and
  was not run against the real service for this release.
