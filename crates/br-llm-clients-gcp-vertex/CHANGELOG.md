# Changelog

All notable changes to `br-llm-clients-gcp-vertex` are documented here. The git tag
`br-llm-clients-gcp-vertex/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

Built on `br-llm-clients-core` 0.5, `br-llm-clients-common-call` 0.2,
`br-llm-clients-api-anthropic-messages` 0.2, `br-llm-clients-common-auth` 0.1 and
`br-llm-clients-common-http` 0.2. Written from the documentation of Google Cloud and
Anthropic read on 2026-10-29; not verified against the real service. Unverified in
particular: the SSE framing and events of `streamRawPredict`, the two error body
shapes (Google for the platform, Anthropic for the model), the request id header
(`request-id` is assumed from the dialect), the top-level `cache_control` on
Vertex, and `retry-after` on a 429.

### Added

- `VertexClient`, a `Client` of `br-llm-clients-core` for Claude on Google Cloud
  Vertex AI: the Anthropic Messages API at `POST
  v1/projects/{project}/locations/{location}/publishers/anthropic/models/{model}:streamRawPredict`,
  streamed. The base is `https://aiplatform.googleapis.com/` for the location
  `global`, `https://aiplatform.{location}.rep.googleapis.com/` for `us` and `eu`,
  and `https://{location}-aiplatform.googleapis.com/` for any other location.
  `VertexClient::new` takes a `ClientSettings`, a `VertexConfig` and an
  `HttpClient`, does no I/O, checks the project, then the location, then builds the
  endpoint and the authorizer, and gives the first failure as a `VertexBuildError`.
  It refuses nothing about the model. It builds one `CallSpec`: the
  `AnthropicMessages` dialect with `ModelPlacement::Omitted` (no `model` in the
  body) and `VersionPlacement::Body("vertex-2023-10-16")` (the member
  `anthropic_version`, no `anthropic-version` header) inside a `Layered` with the
  private hooks of this library. The headers are `content-type`, `accept` and
  `authorization`; no `anthropic-beta` and no `x-goog-user-project`. The call runs
  `run` of common-call and returns its outcome unchanged, with no retry. It is
  `Send`, `Sync` and `Clone`, and its `Debug` shows the model, the project, the
  location and whether a base override is set, never a credential.
- `VertexConfig`: the project, the location, the credential, `with_base` (replaces
  the host, for a Private Service Connect endpoint or a gateway; the route from
  `v1` to `{model}:streamRawPredict` is added to it; a base that is not `https` is
  refused with `InvalidEndpoint`, because the bearer token has the
  `cloud-platform` scope) and `with_id_source`. Building it cannot fail. Its
  `Debug` shows no credential.
- A project is a project id (6 to 30 bytes of lowercase ASCII letters, digits and
  hyphens, starting with a letter and not ending with a hyphen) or a project number
  (1 to 30 ASCII digits). A location is 1 to 32 bytes of lowercase ASCII letters,
  digits and hyphens, starts with a letter, does not end with a hyphen and holds no
  two hyphens in a row.
- `VertexCredential` (`#[non_exhaustive]`) with one case, `ServiceAccount`, which
  holds a `ServiceAccountKey`. Its `Debug` shows the case only.
- `ServiceAccountKey::from_json` reads the JSON text of a Google service account
  key that the caller gives; the library reads no file and no environment
  variable. It refuses with `InvalidCredential` a text that is not a JSON object, a
  `type` other than `service_account`, a `universe_domain` other than
  `googleapis.com`, a `client_email` or `private_key` that is missing, empty or not
  a text, a `private_key` that is not an RSA private key of 2048 to 8192 bits in
  PKCS#8 PEM (read with `rustls-pki-types`, loaded with `ring`), a
  `private_key_id` or `token_uri` that is not a text, and a `token_uri` that is not
  a valid `https` URL. A member set to JSON `null` counts as absent. With no
  `token_uri`, the token URI is `https://oauth2.googleapis.com/token`. The key pair
  is loaded once, so a bad key is refused here and not at the first call. Its
  `Debug` shows the client email and the private key id only.
- The access token comes from the JWT bearer grant (RFC 7523): a JWT signed RS256
  with `ring` at the system time, header `alg`, `typ` and `kid` (the private key
  id; no `kid` when the id is absent or empty), claims `iss` (the client email),
  `scope` (`https://www.googleapis.com/auth/cloud-platform`), `aud` (the token
  URI), `iat` and `exp` (`iat` plus 3600 seconds), posted with
  `TokenForm::jwt_bearer` by the `FormTokenClient` of common-auth to the token URI
  through the same `HttpClient` as the calls. The `expires_in` of the answer sets
  the life of the token in one `TokenCache`; `TokenAuthorizer::bearer` sends it as
  `Authorization: Bearer`. A system clock before 1970 or a signing failure is
  `AuthorizerError::Unavailable`, with a fixed text, and nothing is sent. The
  private key is in the secrets of the `CallSpec`; every assertion and every
  access token is recorded in its `SecretLog` (the assertion before it is posted,
  the token by the observer of the authorizer), so every text of `run` is cleaned
  of them.
- The error code of a refusal is the one of the dialect (`error.type` of an
  Anthropic body) and, when there is none, `error.status` of a Google body
  (`{"error":{"code","message","status"}}`), such as `RESOURCE_EXHAUSTED`, when it
  is 1 to 64 bytes of uppercase ASCII letters and `_`. A 404 whose code is the
  Google status `NOT_FOUND` is `ModelNotFound`; an Anthropic body is left to the
  dialect. Every other hook keeps the behaviour of the dialect and of common-call.
- The declared capabilities: `claude-fable-5-1`, `claude-opus-5-5` and
  `claude-sonnet-5-5` accept `Low` to `Max`, `claude-haiku-5-5` accepts `Off` to
  `Max`, each with 128000 output tokens and no structured output with tools; any
  other id declares everything. Caching is declared allowed (the top-level
  `cache_control` on Vertex is not verified).
- `Endpoint` and `HttpClient` of common-http, the items of the public signatures,
  are re-exported. No error of common-http, common-auth or core crosses
  `VertexBuildError`, `ServiceAccountKey::from_json` or `VertexClient::new`.
- `examples/live_check.rs`, run by hand against the real service: a plain answer,
  streaming, a tool call, usage, and the unknown model id `claude-does-not-exist`,
  which passes only with `ModelNotFound` or `ModelNotAccessible` and prints the
  variant otherwise. It reads `VERTEX_AI_PROJECT_ID`, `VERTEX_AI_LOCATION`,
  `VERTEX_AI_MODEL` and `VERTEX_AI_CREDENTIALS_FILE` (the path of the key file,
  which the example reads) from the environment only, stops before any call when
  one is missing or empty, and never prints a credential.

### Known limits

- A legacy domain-scoped project id (`example.com:proj`) is refused; use the
  project number.
- A location typo is not caught when the client is built; each call then fails
  as a transport error.
- Each `VertexClient` keeps its own token cache: one token fetch per client per
  hour.
- A token is not dropped early when Vertex answers 401; it is used until it
  ends.
- The system clock is used as is for the `iat` of the token assertion.
- Google also answers 404 `NOT_FOUND` for a wrong project or location, or a wrong
  path behind a base override; these too become `ModelNotFound`.
