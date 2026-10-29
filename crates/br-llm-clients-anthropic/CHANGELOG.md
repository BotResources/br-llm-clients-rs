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

- `AnthropicClient`: the `Client` of the Anthropic first-party API (the Messages API on
  `api.anthropic.com`, with streaming), on `br-llm-clients-core` 0.5, `br-llm-clients-common-call`
  0.2 and `br-llm-clients-api-anthropic-messages` 0.2.
  `AnthropicClient::new(ClientSettings, AnthropicConfig, HttpClient)` is not async, does no I/O and
  refuses no model id. It checks the key, then the workspace id, then builds the base, the endpoint
  `{base}/v1/messages` and the authorizer, in this order. `call` runs `run` of common-call on one
  shared `CallSpec` and returns the outcome unchanged. It can be cloned, and a clone shares the
  `CallSpec` and the connections. Its `Debug` shows the model and the endpoint without its query,
  never the key.
- `AnthropicConfig`: the key as a `Secret`, an optional base `Endpoint` (default
  `https://api.anthropic.com/`), an optional workspace id (sent in the header
  `anthropic-workspace-id`), `with_bearer_auth` (the key goes in `authorization: Bearer` instead of
  `x-api-key`), and an optional `IdSource` (default `DefaultIdSource`). Building it cannot fail. Its
  `Debug` never shows the key, shows which header carries the key, and says whether an `IdSource` is
  set.
- `AnthropicBuildError`: `InvalidCredential`, `InvalidEndpoint` and `InvalidWorkspaceId`, each with a
  text `reason` that holds no part of the key. It is `#[non_exhaustive]`.
- The dialect of the call is a `Layered` of `AnthropicMessages` (default options) with private hooks
  that only run `Capabilities::check` with the settings; every other hook keeps its default. The key
  is the one entry of the list of secrets of the call, so a provider text or a `request-id` that
  echoes it is cleaned by `run`. The extra headers hold only `anthropic-workspace-id`, when set.
- The declared capabilities of the current ids `claude-fable-5-1`, `claude-opus-5-5`,
  `claude-sonnet-5-5` and `claude-haiku-5-5`: output limit 128000, the level `Off` refused except
  for `claude-haiku-5-5`, structured output with tools refused, everything else accepted. Any other
  id declares everything and goes to the dialect and to the API.
- The example `live_check`: a by-hand check against the real service, configured by the environment
  variables `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL` (required), `ANTHROPIC_MODEL` (default
  `claude-haiku-5-5`), `ANTHROPIC_WORKSPACE_ID` and `ANTHROPIC_AUTH` (`x-api-key` or `bearer`). It
  checks a plain answer, a streamed answer, a tool round trip, a structured output with three fields
  and the refusal of an unknown model id, and prints the usage of every answer. It reads no file, is
  not part of the API and is not run in CI.
