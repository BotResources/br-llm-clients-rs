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
  arrive, whatever the status. HTTP/1.1 and HTTP/2 are chosen by ALPN; plain
  `http` endpoints are HTTP/1.1 only. Redirects are never followed, proxy
  environment variables and the system certificate store are not read,
  certificate verification cannot be turned off. The roots are the bundled web
  roots plus `extra_root_certificates`. `Accept-Encoding: identity` is always
  sent.
- Connections. While the protocol of a host is unknown only one connection is
  opened for it and concurrent sends wait for its outcome (HTTP/2: they share
  it; HTTP/1.1: each opens its own; a failed attempt is the failure of every send
  that waited for it, and none of them dials again). When every stream of a host's HTTP/2
  connections is in use another connection is opened, up to 16 per host, then a
  send waits within `headers_timeout`, which is one deadline shared by the wait
  and the exchange. A new HTTP/2 connection counts as full until the first
  SETTINGS frame of the server has been read, and no request is handed to it
  before: the sends that arrive meanwhile wait for it, including the send that
  dialed it. A connection whose first SETTINGS frame has not been read within
  `connect_timeout` after the connection is ready is closed. Each send that
  waited for it, and the send that dialed it, counts that as one failed attempt
  (no byte was written) and goes again through the same gate and within the same
  limit of 16 connections, so a server that never sends SETTINGS ends every send
  with `Connect` within three `connect_timeout`s, not one `connect_timeout` per
  send and per attempt in turn. A server limit of zero makes the connection
  full, not unknown: a send then uses or opens another connection. The stream
  limit is read from the SETTINGS frames as they arrive on the wire, entry by
  entry as the bytes stream through (an entry split across reads is carried, no
  frame size is capped and memory stays bounded), so a SETTINGS frame of any
  legal size is read exactly, a change by the server (also a drop to zero) is
  seen at once, and when an entry repeats the last one wins. A first SETTINGS
  frame without a stream-limit entry means no limit; an unreadable frame is
  never read as no limit.
  Addresses of a name are tried with a fixed 250 ms pause between the starts of
  two attempts. HTTP/2 connections send keep-alive pings every 30 s, also when
  idle, and close when a ping gets no answer within 10 s. An idle connection is
  closed after 30 s by a timer, not only when looked at; at most 8 idle
  connections are kept per host, whatever the protocol. These values are fixed.
- Resend. A request is sent again, at most 3 attempts in all, through the same
  gate and the same limit of 16 connections per host as any send, only when the
  stack shows the server did not process it: a pooled connection closed before
  the request was written, an HTTP/2 stream the server refused (REFUSED_STREAM),
  or a request an HTTP/2 connection never took. A remote GOAWAY is never proof
  that a request was not processed: the HTTP/2 library can hand a connection-level
  GOAWAY error to a request whose stream it has already sent (a graceful shutdown
  first announces the largest stream id, which covers every stream), so a request
  that fails on a GOAWAY is `Interrupted` and is not sent again. A GOAWAY retires
  the connection that carried the request; a refused stream does not, the next
  attempt goes through the normal plan and may use the same healthy connection.
  The authorizer is not called again. A request counts as written as soon as the HTTP/2 or HTTP/1.1 library
  has taken it, so a failure after that (a broken pipe, a reset, a close, also
  before the body was read) is never resent and is `Interrupted`. A request an
  HTTP/2 connection never took before it ended is resent, instead of waiting for
  `headers_timeout`, unless the connection ended because the server broke an
  HTTP/2 rule (see `Protocol` below). A GOAWAY that asks for HTTP/1.1 is
  `Protocol` and is not resent. Each attempt has its own connect and headers
  deadlines, so one send makes at most three attempts and can last up to three
  times as long as one attempt; `total_timeout` still bounds the whole send.
- Limits. A response head larger than 64 KiB, or with more than 1024 header
  lines, is refused with `SendError::Protocol`; on HTTP/2 the size is counted as
  RFC 9113 counts a header list, and exactly 64 KiB is accepted.
- `HttpConfig::new` takes `connect_timeout`, `headers_timeout` and
  `idle_timeout` (required, no default, zero refused); `with_total_timeout`,
  `with_user_agent` and `with_extra_root_certificates` (PEM texts) set the
  optional fields.
- `Endpoint` (`https`, `plain_http`, `join_segment`, `with_query_pair`) with the
  strict refusal rules of the intent; a text, or a result of `join_segment` or
  `with_query_pair`, longer than 8192 bytes is `InvalidEndpoint`. `Debug` and
  `Display` never show the query.
- `Secret` (placeholder `Debug` and `Display`, `expose`, never empty).
- `Request` (POST, in-memory body, validated headers, reserved headers refused,
  `User-Agent` included: it is set through `HttpConfig` only); `Debug` shows the
  endpoint, the header names and the body size only.
- `Authorizer`, `AuthorizationRequest`, `AuthorizedHeader` and the ready-made
  `ApiKeyAuthorizer`, `BearerAuthorizer` and `NoAuthorizer`. The authorizer sees
  the exact request, is called once, and its headers replace request headers of
  the same name after validation.
- `Response`, `Headers`, `BodyStream` (`next_chunk`, `poll_next_chunk`,
  `collect`) and `Collected`. Chunks are `bytes::Bytes`, a type of the public
  API. `retry_after` reads delay seconds and the three RFC 9110 date forms.
  `Content-Encoding` other than identity fails the stream at its first read with
  `UnsupportedEncoding`, which never names the encoding (the importer reads the
  header from the response).
- The four timeouts: connect, headers (from the moment a connection is ready,
  covering the wait for a free HTTP/2 stream and the sending of the body), idle
  (each read starts a new wait; time between reads never counts) and total.
- `ConfigError`, `SendError`, `BodyError`, `AuthorizerError` (all
  `#[non_exhaustive]`, every case carries a text). `Connect` and `Tls` come from
  the stage of the connection that failed. A TLS failure after the handshake (a
  corrupt record, an alert) is `Interrupted` when a byte may have been written;
  a server whose HTTP/2 frames break an HTTP/2 rule once the connection is
  established (a SETTINGS value out of range, a SETTINGS frame of a wrong length
  or on a stream, an oversize frame, a header block that cannot be decoded, a
  SETTINGS ACK before any SETTINGS) is `Protocol` for the request that carried
  it, for every send that waited for the first SETTINGS of that connection, and
  for a request that the connection never took; one function classifies it
  whatever the moment the request meets it, and `Connect` stays for a connection
  that was never established or never carried an HTTP/2 SETTINGS frame;
  over TLS a body read until close ends cleanly only with a close_notify. `Connect` also covers a connection
  that could not carry the request when no byte of it had been written;
  `Interrupted` means at least one byte may have reached the server.
- Texts. No text holds a header value, a body or a query, and no text of a lower
  layer is copied: the wording is the library's own, with an HTTP/2 reason or an
  operating-system error kind at most. Every text, `Display` and `Debug` prefix
  included, is at most 512 characters, the text of an `AuthorizerError` too.
  `Debug` of a `Response` shows the status and the number of headers only.
  The exact value of every credential is also removed from every text as a
  second barrier; this is best effort and the guarantee does not rest on it.
- `Collected` and `AuthorizedHeader` are `#[non_exhaustive]`; the crate forbids
  unsafe code.

### Public surface

`Endpoint` (`https`, `plain_http`, `join_segment`, `with_query_pair`; comparable,
`Display`), `Secret` (`new`, `expose`), `Request` (`new`, `with_header`),
`HttpConfig` (`new`, `with_total_timeout`, `with_user_agent`,
`with_extra_root_certificates`; comparable), `HttpClient` (`new`, `send`;
`Debug`), `Response` (`status`, `headers`, `retry_after`, `body_mut`,
`into_parts`), `Headers` (`get`, `get_all`, `get_text`, `iter`), `BodyStream`
(`next_chunk`, `poll_next_chunk`, `collect`), `Collected`, `Authorizer`,
`AuthorizationRequest` (`method`, `path_and_query`, `host`, `headers`, `header`,
`body`), `AuthorizedHeader` (`new`), `BoxFuture`, `ApiKeyAuthorizer` (`new`),
`BearerAuthorizer` (`new`), `NoAuthorizer` (`Default`, `Copy`), `ConfigError`,
`SendError`, `BodyError`, `AuthorizerError` (`unavailable`, `fetch_failed`).
