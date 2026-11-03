# Changelog

All notable changes to `br-llm-clients-aws-bedrock` are documented here. The git tag
`br-llm-clients-aws-bedrock/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

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
