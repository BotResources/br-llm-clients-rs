# Changelog

All notable changes to `br-llm-clients-openai` are documented here. The git tag
`br-llm-clients-openai/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `OpenAiClient`, built by `new` from a `ClientSettings`, an `OpenAiConfig` and an `HttpClient`, implements `Client` for the first-party OpenAI API. Every call is a `POST` to `/v1/responses`, on `api.openai.com` by default or after the optional base of the config, through the dialect `br-llm-clients-api-openai-responses`, with `Authorization: Bearer <key>`. It is `Send`, `Sync` and `Clone`, its `Debug` shows the model and the joined endpoint without its query and no key or header value, builds no `HttpClient`, sets no timeout, does no I/O in `new` and keeps nothing between calls.
- `OpenAiConfig` (with `Debug` and `Clone`; key, `ModelProfile`, optional base `Endpoint`, optional organization, optional project) with the builders `with_base`, `with_organization` and `with_project`. Its `Debug` shows no key and no organization or project value. The organization and the project become the headers `OpenAI-Organization` and `OpenAI-Project`.
- `OpenAiBuildError`, `#[non_exhaustive]` with `Debug`, `Clone`, `PartialEq`, `Eq`, `Display` and `Error`, with the cases `InvalidCredential`, `InvalidOrganization`, `InvalidProject` and `InvalidEndpoint`, each holding a text `reason`: a fixed text for the key, the organization and the project, which never holds the key or a header value, and the text of the HTTP library error for the endpoint. `new` checks the key, the organization, the project and the endpoint in this order, and no refusal of `new` depends on the settings or the model.
- `ModelProfile` (with `Debug`, `Clone` and `PartialEq`; a `Capabilities`, a `ReasoningParameter` and a `ReasoningSummary`) and the presets `gpt_6_astra`, `gpt_6_1_sol`, `gpt_6_luna`, `gpt_5_6_sol`, `gpt_5_5`, `gpt_5_4`, `gpt_5_4_mini` and `gpt_4_1_mini`, with the levels, the output ceilings and the cache flag of the model pages. Every preset has the summary `None`; `with_summary` changes it.
- Every call checks the `Capabilities` of the profile before sending and gives the refusals of `br-llm-clients-core`; this library adds no refusal of its own.
- Two error rows before the table of the dialect: status 404 with the code `model_not_found` is `ModelNotFound`, and the code `context_length_exceeded` is `ContextTooLong` without a limit.
- The request id of the report is not given when the `x-request-id` value contains the key.
- Re-exports of `ReasoningSummary`, `ReasoningParameter`, `Capabilities`, `CapabilitiesSpec`, `CapabilitiesError` and `Accepted`.
- The example `live_check` runs three checks against the real service with a key from `OPENAI_API_KEY` and a model from `OPENAI_MODEL` (default `gpt-5.4-mini`): a simple conversation, a weather tool called twice, and a structured output with three fields.
