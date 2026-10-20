# Changelog

All notable changes to `br-llm-clients-kimi` are documented here. The git tag
`br-llm-clients-kimi/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `KimiClient`, a `Client` of `br-llm-clients-core` for the Kimi models of Moonshot
  AI. `KimiClient::new` takes a `ClientSettings`, a `KimiConfig` and an
  `HttpClient`, and gives a `KimiClient` or a `KimiBuildError`. The call runs
  `common_call::run` with the Chat Completions dialect and returns its outcome
  unchanged. `thinking` gives the `KimiThinking` of the client, or none for an
  unknown model. It is `Send`, `Sync` and `Clone`, and its `Debug` shows the model,
  the thinking kind and the base, never the key.
- `KimiConfig`: the key as a `Secret`, `with_base` (default
  `https://api.moonshot.ai`, a path in the base is kept and the route `v1`, `chat`,
  `completions` is added to it) and `with_thinking`. Its `Debug` shows the base and
  the thinking kind, never the key.
- `KimiThinking` (`EffortScale`, `ThinkingSwitch`, `AlwaysThinking`, `#[non_exhaustive]`),
  with the table of the four current models: `kimi-k3`, `kimi-k2.6`,
  `kimi-k2.7-code` and `kimi-k2.7-code-highspeed`. The `Capabilities` of the client
  depend on the kind: `Off` is refused with `ReasoningNotSupported` on `EffortScale`
  and `AlwaysThinking`, and every other field is the one of
  `CapabilitiesSpec::everything()`.
- `KimiBuildError` (`InvalidCredential`, `InvalidEndpoint`, `InvalidDialectOptions`,
  `#[non_exhaustive]`, each with a text `reason`). A key with a character outside
  `!` to `~` is refused at construction, and the reason never holds the key.
- A model that the table does not list and that has no `KimiThinking` in the config
  is accepted by `new`; every call to it is refused with `ProviderRejected`, the code
  `kimi.unknown_model` and no status, and nothing is sent.
- The rewrite of the body of the Chat Completions dialect: `reasoning_effort` on
  `EffortScale` only (`Low` to `low`, `Medium` and `High` to `high`, `Extra` and
  `Max` to `max`); the member `thinking` (`disabled` for `Off` on `ThinkingSwitch`,
  `enabled` otherwise) on the two other kinds; the thinking of the turn in progress
  sent back as `reasoning_content` when thinking is on; the member `name` on each
  `tool` message.
- A request id header value that holds the key is not reported.
- `examples/live_check.rs`, the one test of the library: three checks against the
  real service (a simple conversation, a weather tool called twice, a structured
  output with three fields). It reads `MOONSHOT_API_KEY`, and `MOONSHOT_MODEL`
  (default `kimi-k2.6`), from the environment or from a `.env` file.
