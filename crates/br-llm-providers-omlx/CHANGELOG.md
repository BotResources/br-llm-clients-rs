# Changelog

All notable changes to `br-llm-providers-omlx` are documented here. The git tag
`br-llm-providers-omlx/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `OmlxClient`, the `Client` of `br-llm-clients-core` for one oMLX server, on
  its chat route `/v1/chat/completions` under the path of the base.
  `OmlxClient::new` takes a `ClientSettings`, an `OmlxConfig` and an
  `HttpClient`, does no I/O and gives an `OmlxBuildError` or a client. It
  checks, in this order, the key, the extra headers, the endpoint of the route
  and the chat options, and returns the first failure. It is `Send`, `Sync` and
  `Clone`, holds one `CallSpec` behind an `Arc`, does not retry and has a
  hand-written `Debug` that shows no key, no header value and no extra body
  member.
- `OmlxConfig`: the base `Endpoint`, the `ReasoningTable` and the
  `Capabilities`, with `with_key`, `with_extra_body`, `with_extra_headers`,
  `with_reasoning_carry_back` and `with_id_source`. The configuration has no
  key, sends the reasoning back, and uses the `DefaultIdSource` until the
  operator sets them. The constructors check nothing.
- The library builds the chat options: the reasoning field `reasoning_content`,
  the reasoning carry-back field `reasoning_content` when the reasoning is sent
  back, and the extra body of the configuration.
- The key, when set, goes in `Authorization: Bearer`. With no key the request
  holds no key header. The key is the only secret removed from the texts of the
  outcome.
- `OmlxBuildError` (`#[non_exhaustive]`) with the cases `InvalidCredential`,
  `InvalidExtraHeader`, `InvalidEndpoint` and `InvalidDialectOptions`, each
  with a `reason` that never holds the key, a header value or a body.
- The hooks run the `Capabilities` check before the chat dialect encodes, and
  give the response header `X-oMLX-Request-ID` as the request id.
- Errors: a 404 with the code `not_found_error` is `ModelNotFound`; a 400 with
  the code `context_length_exceeded` is `ContextTooLong`, with the limit read
  from the message; a 409 is `Unavailable` with no delay. Every other error is
  left to the chat dialect and the generic table of
  `br-llm-clients-common-call`.
- The example `examples/live_check.rs`, a developer tool that is not run in CI,
  driven by `OMLX_BASE_URL`, `OMLX_MODEL`, `OMLX_API_KEY`, `OMLX_STRUCTURED`
  and `OMLX_MODEL_FALLBACK`. Its nine checks include the request id, the reuse
  of the prefix cache and an unknown model name.
- The example `examples/prefix_cache.rs`: one conversation of four calls, run
  with and without the reasoning sent back, that prints the share of each
  prompt read from the prefix cache.
- Re-exports: `Endpoint`, `Secret`, `HttpClient`, `IdSource`,
  `DefaultIdSource`, `ReasoningTable`, `ReasoningRule`, `ReasoningEffort`,
  `Map`, `Value`, `Capabilities`, `CapabilitiesSpec`, `Accepted`,
  `CapabilitiesError` and `ReasoningLevel`.
