# Changelog

All notable changes to `br-llm-clients-anthropic` are documented here. The git tag
`br-llm-clients-anthropic/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `AnthropicClient`: the `Client` of the Anthropic first-party API (the Messages API, with
  streaming). `AnthropicClient::new(ClientSettings, AnthropicConfig, HttpClient)` does no I/O and
  refuses no model id. `call` runs `br-llm-clients-common-call` on one shared `CallSpec`. It can be
  cloned, and a clone shares the `CallSpec` and the connections. Its `Debug` shows the model and the
  endpoint without its query, never the key.
- `AnthropicConfig`: the key as a `Secret`, an optional base `Endpoint` (default
  `https://api.anthropic.com/`; the route `v1/messages` is added to it) and an optional workspace id
  (sent in the header `anthropic-workspace-id`). Building it cannot fail.
- `AnthropicBuildError`: `InvalidCredential`, `InvalidEndpoint` and `InvalidWorkspaceId`, each with a
  text `reason` that holds no part of the key. It is `#[non_exhaustive]`.
- The key goes out in the one header `x-api-key`. The raw key is in the list of secrets of the call,
  so that a provider text that echoes it is cleaned. A `request-id` header that contains the key
  gives no request id.
- The declared capabilities of the current ids `claude-fable-5-1`, `claude-opus-5-5`,
  `claude-sonnet-5-5` and `claude-haiku-5-5`: output limit 128000, `Off` refused except for
  `claude-haiku-5-5`, structured output with tools refused. Any other id declares everything and goes
  to the dialect and to the API. The refusals come from `Capabilities::check` on each call.
- The example `live_check`: a by-hand check against the real service with `ANTHROPIC_API_KEY` and
  optional `ANTHROPIC_MODEL` (default `claude-haiku-5-5`): a simple conversation, a conversation
  with a tool called twice, and a structured reply. It is not part of the API and not run in CI.
