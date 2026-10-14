# Changelog

All notable changes to `br-llm-clients-common-http` are documented here. The git tag
`br-llm-clients-common-http/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `HttpClient`, built once from an `HttpConfig` (`HttpClient::new`), cloneable,
  `Send` and `Sync`; a clone shares the connections. `send(Request, &dyn
  Authorizer)` makes one HTTP POST and gives a `Response` as soon as the headers
  arrive, whatever the status. HTTP/1.1 and HTTP/2 are chosen by ALPN; several
  requests of one client share one HTTP/2 connection. Redirects are never
  followed, proxy environment variables and the system certificate store are not
  read, certificate verification cannot be turned off. The roots are the bundled
  web roots plus `extra_root_certificates`. `Accept-Encoding: identity` is always
  sent. A response head above 64 KiB is refused with `SendError::Protocol`.
- `HttpConfig`: `connect_timeout`, `headers_timeout` and `idle_timeout` (required,
  no default, zero refused), optional `total_timeout`, `user_agent` and
  `extra_root_certificates` (PEM texts).
- `Endpoint` (`https`, `plain_http`, `join_segment`, `with_query_pair`) with the
  strict refusal rules of the intent; `Debug` and `Display` never show the query.
- `Secret` (placeholder `Debug` and `Display`, `expose`, never empty).
- `Request` (POST, in-memory body, validated headers, reserved headers refused, `User-Agent` included: it is set through `HttpConfig` only);
  `Debug` shows the endpoint, the header names and the body size only.
- `Authorizer`, `AuthorizationRequest`, `AuthorizedHeader` and the ready-made
  `ApiKeyAuthorizer`, `BearerAuthorizer` and `NoAuthorizer`. The authorizer sees
  the exact request, is called once, and its headers replace request headers of
  the same name after validation.
- `Response`, `Headers`, `BodyStream` (`next_chunk`, `poll_next_chunk`,
  `collect`) and `Collected`. `retry_after` reads delay seconds and the three
  RFC 9110 date forms. `Content-Encoding` other than identity fails the stream at
  its first read with `UnsupportedEncoding`, which never names the encoding (the
  importer reads the header from the response).
- The four timeouts: connect, headers (from the moment a connection is ready,
  covering the wait for a free HTTP/2 stream and the sending of the body), idle
  (only while the importer is reading) and total.
- `ConfigError`, `SendError`, `BodyError`, `AuthorizerError` (all
  `#[non_exhaustive]`, every case carries a text). `Connect` and `Tls` come from
  the stage of the connection that failed. No text holds a header value, a body or
  a query; the exact value of every credential is also removed from every text.
