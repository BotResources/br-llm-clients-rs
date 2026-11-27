# Changelog

All notable changes to `br-llm-clients-anthropic` are documented here. The git tag
`br-llm-clients-anthropic/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.2.0

Opens the line 0.2: an operator can choose the lifetime of the prompt cache, five minutes (the
default, the behaviour of 0.1) or one hour. Built on `br-llm-clients-api-anthropic-messages` 0.3.

### Added

- `CACHE_ENV`, the text `ANTHROPIC_CACHE`: the name of the variable that holds the cache setting
  of the client. The library reads no environment variable: the runner reads it when it starts
  and gives its text to `AnthropicCache::parse`.
- `AnthropicCache`: the cache options of the client. `AnthropicCache::new` takes a `MessagesCacheOptions`.
  `AnthropicCache::parse` takes the text of the
  variable, or nothing. Nothing, an empty text and a blank text give the default, five minutes,
  the behaviour of the previous line. Any other text is a JSON object whose only member is
  `messages`, optional, parsed by `MessagesCacheOptions::from_json` of the dialect; for example
  `{"messages":{"ttl":"1h"}}`. An absent `messages` gives the default of the dialect.
  `messages` gives the `MessagesCacheOptions`. `Default` gives five minutes.
- `CacheTtl` and `MessagesCacheOptions` of the dialect are re-exported, the items of the
  signatures of the cache type.
- `AnthropicCacheError` (`#[non_exhaustive]`): `NotJson`, `NotAnObject`, `UnknownMember` with the
  member, and `InvalidMessages` with the member of the section `messages` that the dialect
  refused, when there is one. Its text names the variable and the member, and shows no value.
- `AnthropicConfig::with_cache` takes an `AnthropicCache`. A config built without it holds the
  default.
  The `Debug` of the config shows the cache options.
- `examples/live_check` runs a sixth check, (f) the cache read with the lifetime of one hour:
  the same request of more than 4096 tokens is sent twice with `CacheRequirement::Required` and
  a config built with `CacheTtl::OneHour`. It passes when the API accepts both
  calls and the second reports a cache read above zero.
- The Helm library chart `br-llm-clients-anthropic-chart` 0.1.0, in
  `charts/br-llm-clients-anthropic-chart` of this repository. It depends on
  `br-llm-clients-api-anthropic-messages-chart` 0.1.0 under the alias `messages`, and its named
  template `br-llm-clients-anthropic-chart.env` renders the container env entry
  `ANTHROPIC_CACHE` with the text `{"messages": …}`, or nothing when no value is set.

### Changed

- The dialect is `AnthropicMessages::with_options` with the default placements and the cache
  lifetime of the config. `CacheRequirement::Required` asks for that lifetime: the top-level
  `cache_control` is `{"type":"ephemeral","ttl":"1h"}` for one hour. With the default, every
  request is the one of 0.1.
- Depends on `br-llm-clients-api-anthropic-messages` 0.3 and on `serde_json` 1.

## 0.1.1

### Changed

- Pins `br-llm-clients-api-anthropic-messages` 0.2.1: every adaptive `thinking` is sent with
  `"display": "summarized"`, so the models whose default omits the thinking text answer with a
  summary of their thinking.

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
