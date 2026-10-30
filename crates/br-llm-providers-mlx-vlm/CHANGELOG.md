# Changelog

All notable changes to `br-llm-providers-mlx-vlm` are documented here. The git tag
`br-llm-providers-mlx-vlm/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `MlxVlmClient`, the `Client` of `br-llm-clients-core` for one mlx-vlm server, on
  its chat route `/v1/chat/completions` under the path of the base.
  `MlxVlmClient::new` takes a `ClientSettings`, an `MlxVlmConfig` and an
  `HttpClient`, does no I/O and gives an `MlxVlmBuildError` or a client. It
  checks, in this order, the key, the extra headers, the endpoint of the route
  and the chat options, and returns the first failure. It is `Send`, `Sync` and
  `Clone`, holds one `CallSpec` behind an `Arc`, does not retry and has a
  hand-written `Debug` that shows no key, no header value and no extra body
  member.
- `MlxVlmConfig`: the base `Endpoint`, the `ReasoningTable` and the
  `Capabilities`, with `with_key`, `with_extra_body`, `with_extra_headers`,
  `with_reasoning_carry_back` and `with_id_source`. The configuration has no
  key, sends the reasoning back, and uses the `DefaultIdSource` until the
  operator sets them. The constructors check nothing.
- The library builds the chat options: the reasoning field `reasoning_content`,
  the reasoning carry-back field `reasoning_content` when the reasoning is sent
  back, the token limit member `max_tokens` (mlx-vlm does not read
  `max_completion_tokens`), and the extra body of the configuration.
- The key, when set, goes in `Authorization: Bearer`. With no key the request
  holds no key header. The key is the only secret removed from the texts of the
  outcome.
- `MlxVlmBuildError` (`#[non_exhaustive]`) with the cases `InvalidCredential`,
  `InvalidExtraHeader`, `InvalidEndpoint` and `InvalidDialectOptions`, each
  with a `reason` that never holds the key, a header value or a body.
- The hooks run the `Capabilities` check before the chat dialect encodes. The
  request id is the rule of the chat dialect; mlx-vlm sets none on the chat
  route.
- Errors: a 400 whose `detail` starts with `Request needs ` and holds
  `, but MAX_KV_SIZE is ` is `ContextTooLong`, with the limit read from the
  text. Every other error is left to the chat dialect and the generic table of
  `br-llm-clients-common-call`.
- The example `examples/live_check.rs`, a developer tool that is not run in CI,
  driven by `MLX_VLM_BASE_URL`, `MLX_VLM_MODEL` and `MLX_VLM_API_KEY`. Its
  eight checks include the token limit and the reuse of the prefix cache.
- The example `examples/prefix_cache.rs`: one conversation of four calls, run
  with and without the reasoning sent back, that prints the share of each
  prompt read from the prefix cache.
- Re-exports: `Endpoint`, `Secret`, `HttpClient`, `IdSource`,
  `DefaultIdSource`, `ReasoningTable`, `ReasoningRule`, `ReasoningEffort`,
  `Map`, `Value`, `Capabilities`, `CapabilitiesSpec`, `Accepted`,
  `CapabilitiesError` and `ReasoningLevel`.
