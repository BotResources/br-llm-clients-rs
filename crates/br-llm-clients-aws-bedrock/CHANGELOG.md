# Changelog

All notable changes to `br-llm-clients-aws-bedrock` are documented here. The git tag
`br-llm-clients-aws-bedrock/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.3.0

Opens the line 0.3: an operator can choose the lifetime of the prompt cache, five minutes (the
default, the behaviour of 0.2) or one hour. Built on `br-llm-clients-api-anthropic-messages` 0.3.

### Added

- `CACHE_ENV`, the text `AWS_BEDROCK_CACHE`: the name of the variable that holds the cache setting
  of the client. The library reads no environment variable: the runner reads it when it starts
  and gives its text to `BedrockCache::parse`.
- `BedrockCache`: the cache options of the client. `BedrockCache::new` takes a `MessagesCacheOptions`.
  `BedrockCache::parse` takes the text of the
  variable, or nothing. Nothing, an empty text and a blank text give the default, five minutes,
  the behaviour of the previous line. Any other text is a JSON object whose only member is
  `messages`, optional, parsed by `MessagesCacheOptions::from_json` of the dialect; for example
  `{"messages":{"ttl":"1h"}}`. An absent `messages` gives the default of the dialect.
  `messages` gives the `MessagesCacheOptions`. `Default` gives five minutes.
- `CacheTtl` and `MessagesCacheOptions` of the dialect are re-exported, the items of the
  signatures of the cache type.
- `BedrockCacheError` (`#[non_exhaustive]`): `NotJson`, `NotAnObject`, `UnknownMember` with the
  member, and `InvalidMessages` with the member of the section `messages` that the dialect
  refused, when there is one. Its text names the variable and the member, and shows no value.
- `BedrockConfig::with_cache` takes a `BedrockCache`. A config built without it holds the default.
  The `Debug` of the config shows the cache options.
- `examples/live_check.rs` runs a seventh check, the cache read with the lifetime of one hour:
  the same request of more than 4096 tokens is sent twice with `CacheRequirement::Required` and
  a config built with `CacheTtl::OneHour`. It passes when the API accepts both
  calls and the second reads the cache. For a model whose Cache is no, it passes when the call is
  refused before sending with `CacheNotSupported`, as the cache read of 0.2 does.
- The Helm library chart `br-llm-clients-aws-bedrock-chart` 0.1.0, in
  `charts/br-llm-clients-aws-bedrock-chart` of this repository. It depends on
  `br-llm-clients-api-anthropic-messages-chart` 0.1.0 under the alias `messages`, and its named
  template `br-llm-clients-aws-bedrock-chart.env` renders the container env entry
  `AWS_BEDROCK_CACHE` with the text `{"messages": …}`, or nothing when no value is set.

### Changed

- The dialect is `AnthropicMessages::with_options` with the default placements (the model in
  the body, the version in the header) and the cache lifetime of the config. With Cache yes,
  `CacheRequirement::Required` makes Bedrock cache the prompt for that lifetime, five minutes or
  one hour: the top-level `cache_control` is `{"type":"ephemeral","ttl":"1h"}` for one hour.
  With the default, every request is the one of 0.2. With Cache no, `Required` is still refused
  before sending with `CacheNotSupported`, whatever the lifetime.
- Depends on `br-llm-clients-api-anthropic-messages` 0.3 and on `serde_json` 1, which was a
  dev-dependency.
- A cache of one hour is no longer outside the scope of the library. Explicit cache breakpoints
  still are.

## 0.2.0

Opens the line 0.2. The public API is unchanged; the declared capabilities of the models
change, so a request that 0.1 accepted may be refused before sending, and the reverse.

### Changed

- The declared capabilities are one explicit table, looked up by the base name of the
  model id (one inference profile prefix and one `anthropic.` removed), with a row for each
  of the seven models that `/anthropic/v1/messages` serves:

  | Base name | Accepted levels | Largest output tokens | Cache |
  | --- | --- | --- | --- |
  | `claude-fable-5-1` | `Low` to `Max` | 128000 | no |
  | `claude-opus-5-5` | `Low` to `Max` | 128000 | yes |
  | `claude-sonnet-5-5` | `Low` to `Max` | 128000 | yes |
  | `claude-haiku-5-5` | `Off` to `Max` | 128000 | yes |
  | `claude-opus-4-8` | `Off` to `Max` | 128000 | yes |
  | `claude-opus-4-7` | `Off` to `Max` | 128000 | yes |
  | `claude-haiku-4-5-20251001-v1:0` | `Off` only | 64000 | yes |

  No row accepts structured output with tools, because the dialect refuses that pair.
  `claude-opus-4-8`, `claude-opus-4-7` and `claude-haiku-4-5-20251001-v1:0` were declared
  with everything in 0.1; they now refuse a reasoning level, a largest output and
  structured output with tools that they do not accept, before sending.
- With Cache yes, `CacheRequirement::Required` is accepted, and the dialect sends the
  top-level `cache_control`, so Bedrock caches the prompt for 5 minutes up to its last
  block that can be cached. With Cache no, and for any id outside the table, it is still
  refused before sending with `CacheNotSupported`.

### Added

- `examples/live_check.rs` runs a sixth check, the cache read: the same request of more
  than 4096 tokens is sent twice with `CacheRequirement::Required`, and the check passes
  when the second call reports a cache read above zero. For a model whose Cache is no, it
  passes when the call is refused before sending with `CacheNotSupported`.

## 0.1.2

### Changed

- Pins `br-llm-clients-api-anthropic-messages` 0.2.1: every adaptive `thinking` is sent with
  `"display": "summarized"`, so the models whose default omits the thinking text answer with a
  summary of their thinking.

## 0.1.1

### Changed

- `aws-sigv4` moves from `~1.3` to `~1.6.2` and `aws-credential-types` from `~1.2`
  to `~1.3.1`. `aws-smithy-http` 0.64.2 and `aws-smithy-types` 1.9 under them no
  longer depend on `pin-utils`, which RUSTSEC-2026-0441 flags as unmaintained, so
  the advisory leaves the dependency tree of this crate. The signed headers are
  unchanged: the same request, credentials and time give the same `authorization`,
  `x-amz-date` and `x-amz-security-token` on 1.3.8 and 1.6.4.
- The minimum supported Rust version of this crate rises from 1.89 to 1.94.1, the
  one of `aws-sigv4` 1.6. The other crates of the repository stay on 1.89.

## 0.1.0

Built on `br-llm-clients-core` 0.5, `br-llm-clients-common-call` 0.2,
`br-llm-clients-api-anthropic-messages` 0.2 and `br-llm-clients-common-http` 0.2.
Written from the documentation of AWS and Anthropic read on 2026-10-29; not verified
against the real service.

### Added

- `BedrockClient`, a `Client` of `br-llm-clients-core` for Claude on Amazon Bedrock:
  the Anthropic Messages API at `POST /anthropic/v1/messages` of the host
  `bedrock-runtime.{region}.amazonaws.com`, streamed. `BedrockClient::new` takes a
  `ClientSettings`, a `BedrockConfig` and an `HttpClient`, does no I/O, checks the
  region, then the credential, then builds the base, the endpoint and the
  authorizer, and gives the first failure as a `BedrockBuildError`. It builds one
  `CallSpec`: the `AnthropicMessages` dialect with its default options inside a
  `Layered` with the private hooks of this library. The call runs `run` of
  common-call and returns its outcome unchanged, with no retry. It is `Send`,
  `Sync` and `Clone`, and its `Debug` shows the model, the region and whether a
  base override is set, never a credential.
- `BedrockConfig`: the region, the credential, `with_base` (replaces the host, for
  a VPC endpoint or a gateway; the route `anthropic`, `v1`, `messages` is added to
  it) and `with_id_source`. Building it cannot fail. Its `Debug` shows no
  credential.
- `BedrockCredential` (`#[non_exhaustive]`): `ApiKey`, a Bedrock API key sent in
  `x-api-key` by the `ApiKeyAuthorizer` of common-http and listed in the secrets
  of the `CallSpec`; `Sigv4`, a `CredentialSource` asked once per request, whose
  request is signed with AWS Signature Version 4 (the `aws-sigv4` crate, service
  name `bedrock`, the region of the config, the system time). The signature adds
  `authorization`, `x-amz-date` and, with a session token, `x-amz-security-token`,
  as `Secret` values; the secret access key and the session token are recorded in
  the `SecretLog` of the `CallSpec`, so every text of `run` is cleaned of them.
  The signer runs with no tracing dispatcher, so its trace events, which hold the
  canonical request and so the session token, are dropped. A binary must not
  enable the `tracing` feature `log-always`: with it, `aws-sigv4` sends that trace
  event to the `log` crate as well, and this crate cannot stop it. Every signing
  failure is `AuthorizerError::Unavailable`, which is not retried; no failure of
  the signer is a `FetchFailed`. The `Debug` shows the case only.
- `AwsCredentials` (an access key id, a secret access key and an optional session
  token; `AwsCredentials::new` refuses an access key id that is empty or not
  visible ASCII with `InvalidCredential`, and the signer checks it again; the
  `Debug` shows no value, not even the access key id),
  `CredentialSource` (a `Send + Sync` trait whose async `credentials` gives
  `AwsCredentials` or an `AuthorizerError`; a failure stops the request before it
  is sent) and `StaticCredentials`. `AuthorizerError`, `BoxFuture`, `Endpoint`,
  `HttpClient` and `Secret` of common-http, the items of its public signatures,
  are re-exported.
- `BedrockBuildError` (`InvalidRegion`, `InvalidCredential`, `InvalidEndpoint`,
  `#[non_exhaustive]`, each with a text `reason` that never holds a credential).
- The request id of a report is the one of the dialect and, when there is none,
  the value of `x-amzn-requestid`.
- The declared capabilities, looked up by the base name of the model id (one
  inference profile prefix and `anthropic.` removed): `claude-fable-5-1`,
  `claude-opus-5-5` and `claude-sonnet-5-5` accept `Low` to `Max`,
  `claude-haiku-5-5` accepts `Off` to `Max`, each with 128000 output tokens and no
  structured output with tools; any other id declares everything. Caching is false
  for every id, so `CacheRequirement::Required` is refused before sending with
  `CacheNotSupported`.
- `examples/live_check.rs`, run by hand against the real service: a plain answer,
  streaming, a tool call, usage, and an unknown model id, which passes only with
  `ModelNotFound`, `ModelNotAccessible` or `ProviderRejected`. With both a Bedrock
  API key and an access key pair set, it runs the five checks once per path,
  labelled `sigv4` and `api-key`. It reads `AWS_REGION`
  (required), `BEDROCK_MODEL` (default
  `eu.anthropic.claude-haiku-4-5-20251001-v1:0`), `AWS_BEARER_TOKEN_BEDROCK` or
  `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and `AWS_SESSION_TOKEN`, and
  `BEDROCK_ENDPOINT` from the environment only, and never prints a credential.
