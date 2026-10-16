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
  it; HTTP/1.1: each opens its own). A dial that fails (name resolution, TCP,
  TLS or `connect_timeout`) is the outcome of every send that waited for it,
  also when the protocol of the host is known and the dial was for one more
  connection: each of them fails with that one failure, of one kind, in about
  the time of one dial, and none of them dials again. A waiter reads the failure
  under the lock of the pool when it plans, so a wake for an unrelated change
  (any stream freed on the host) never makes it miss the failure and dial again.
  A send that arrives after the failure was told is not affected by it and dials
  for itself. A send that waits for a dial keeps no timer of its own: the dial
  always ends (it registers, it fails, its send is dropped and the gate opens,
  or its task panics and the waiters get one `Connect` failure), so the waiters
  get its outcome, also a success or a `ConnectTimeout`
  that falls a few milliseconds after the connect timeout the waiters would have
  counted from their own start. The waiters are not served in the order they
  arrived: a send without `total_timeout` that waits while other sends start
  dials and are dropped again and again can wait for a long time, and
  `total_timeout` bounds it. `headers_timeout` bounds the wait of a send that
  waits for a free stream. A dial makes
  progress whether or not the send that started it is polled again: it runs in a
  task that the send owns and that stops when the send is dropped, so a send
  that started a dial and is never polled again does not hold up the other
  sends. A send that is alive but never polled again keeps the stream of the
  connection its dial opened (the connection is not idle, its idle timer does
  not fire, and a connection with a limit of one stays full) until it is polled
  again or dropped; the other sends are not blocked, they dial more connections,
  up to 16. A dial task that panics tells its waiters one `Connect` failure, and
  none of them dials again. When every stream of a host's HTTP/2 connections is
  in use another connection is opened, up to 16 per host, then a send waits
  within `headers_timeout`, which is one deadline per attempt: it restarts when
  the send dials again, and runs from the moment a connection is ready. A new
  HTTP/2 connection counts as full until the first SETTINGS frame of the server
  has been read, and no request is handed to it before: the sends that arrive
  meanwhile wait for it, including the send that dialed it. A connection whose
  first SETTINGS frame has not been read within `connect_timeout` after the
  connection is ready is closed.
- Attempts of the sends that wait. Every connection that a send waited for
  (the dial in flight, the first SETTINGS of a connection, or the dial of a new
  connection while every stream is in use) and then cannot use costs that send
  exactly one of its three attempts, whatever the reason: the connection never
  sent SETTINGS, ended before or after them (EOF, reset, GOAWAY), was retired,
  or kept a limit of zero since its first SETTINGS. This holds however late the
  send wakes: a connection is retired before the pool lets it go, whichever of
  the pool and the task that follows the connection sees its end first, and a
  send reads the state of the connection it waited for after it has read the
  rest of the pool, so a wake that comes while the connection ends, or while its
  first SETTINGS are read, never costs the send one more connection or one more
  wait. The send does not dial its
  own connection for it, neither when the connection opened and could not be
  used, nor when the dial failed (then the send fails with the failure of that
  dial, see Connections). So connections stay in the order of the attempts, not
  of the sends: a server that never sends SETTINGS, that announces a limit of
  zero, or that closes or resets right after its SETTINGS ends every send with
  `Connect` after three attempts on a handful of connections. A connection that
  breaks an HTTP/2 rule is `Protocol` for every send that waited for it. A limit
  that a server lowers to zero after a usable first SETTINGS costs no attempt,
  also when the two frames come in the same read: the request is handed to the
  connection and waits behind the limit, bounded by `headers_timeout`. A first
  SETTINGS of zero that a later SETTINGS raises makes the connection usable for
  every send that starts after the raise. A first zero that a SETTINGS frame
  read in the same read raises means the limit was never zero for anyone: the
  sends that waited lose nothing. A send that waited and had already looked at
  the connection while its limit was zero keeps the attempt it spent. A
  connection that announced zero stays pooled until it is idle for 30 s and is
  never trimmed to keep 8 idle ones, so the 16 connections of a host cap also
  holds for them; sends beyond it wait within `headers_timeout` and end with
  `HeadersTimeout`. The connect deadline starts with each dial the send itself
  makes and never counts the time spent behind the dial of another send, which
  has no deadline of its own. A host leaves the pool when its last connection
  has ended and nothing uses it; the host of an HTTP/1 connection that ended
  while a response was still being read stays until the next send to any host
  looks at the pool.
- Settings and limits. A server limit of zero makes the connection full, not
  unknown. The stream limit is read from the SETTINGS frames as they arrive on
  the wire, entry by
  entry as the bytes stream through (an entry split across reads is carried, the
  library caps no frame size itself and memory stays bounded), so a SETTINGS
  frame of any size the HTTP/2 library accepts is read exactly (that library
  refuses a frame above 16384 bytes, which is `Protocol`), a change by the
  server (also a drop to zero) is seen at once, and when an entry repeats the
  last one wins; several frames read together are applied in order, and the
  result of the whole read is published at once, so no other send sees a zero
  that the same read raises. A first SETTINGS frame without a stream-limit
  entry means no limit. A SETTINGS frame whose length is not a whole number of
  entries is not read as a limit at all (the HTTP/2 library closes the
  connection for it, which is `Protocol`). A limit is applied only once the
  HTTP/2 library has accepted the bytes that carried it, so a server that
  announces a limit, even zero, and then breaks a rule is `Protocol` for every
  send, not `Connect`.
  Addresses of a name are tried with a fixed 250 ms pause between the starts of
  two attempts. HTTP/2 connections send keep-alive pings after 30 s without
  anything received from the server, also when idle, and close when a ping gets
  no answer within 10 s. An idle connection is closed after 30 s by a timer, not
  only when looked at; at most 8 idle connections are kept per host, whatever
  the protocol. These values are fixed.
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
  The authorizer is not called again. A request counts as taken as soon as the
  HTTP/2 or HTTP/1.1 library has taken it. A failure after that is never resent,
  except a refused HTTP/2 stream: it is `Interrupted` (a broken pipe, a reset, a
  close, a GOAWAY, also before the body was read) or `Protocol` (the server
  broke an HTTP/2 rule). When the three attempts are used, the failure is
  `Interrupted` if any attempt was taken by a connection and `Connect` if none
  was. A request an
  HTTP/2 connection never took before it ended is resent, instead of waiting for
  `headers_timeout`, unless the connection ended because the server broke an
  HTTP/2 rule (see `Protocol` below). A GOAWAY that asks for HTTP/1.1 is
  `Protocol` and is not resent. Each attempt has its own connect and headers
  deadlines, so one send makes at most three attempts and can last up to three
  times as long as one attempt; `total_timeout` still bounds the whole send.
- Bodies. A body that ends before its declared length is `Interrupted` on
  HTTP/1.1 and on HTTP/2 alike, also when an HTTP/2 stream is reset with
  NO_ERROR before the declared bytes came (the HTTP/2 library reads such a reset
  as a clean end; the library compares what arrived with the declared length),
  and also for the HTTP/2 stream reset that follows a body shorter than its
  length. On HTTP/2 a frame that goes past the declared length is `Interrupted`
  when declared bytes were still missing before that frame, and `Protocol` when
  the whole length had already been delivered. A stream reset that the HTTP/2
  library itself makes with PROTOCOL_ERROR while declared bytes are missing is
  read as that cut, as for a frame that goes past the declared length and for a
  pseudo header in trailers. On HTTP/1.1 a body longer than its declared length
  is cut silently at the declared length. An HTTP/2 stream reset with NO_ERROR,
  or a GOAWAY with NO_ERROR that excludes the stream, when no length was
  declared cannot be told from an end, so the importer relies on its own end
  marker (with a declared length both are `Interrupted`). A response whose
  HEADERS frame ends the stream while declaring a length above zero is a
  `Protocol` failure of the send, other than a 204 or a 304. A 204 or a 304 is a
  response without a body whatever length it declares and whatever frames follow
  its headers (HTTP/2 allows empty DATA or trailers after a head that declared a
  length; the HTTP/2 library rejects them, so the library does not read the body
  of such a response). One function builds every `Protocol` text of a send, a
  text that names the fault first and the endpoint after it so that a long path
  cannot crowd the fault out, whether the fault meets a waiter or the request.
  The `Connect` and `Tls` texts of a dial end the same way, with the endpoint in
  parentheses: `could not connect: connection refused (endpoint)` and `name
  resolution failed: kind (endpoint)` name the operating-system error kind after
  a colon; the certificate texts (`the server certificate has an unknown issuer
  (endpoint)`) and the alert text carry no kind; a handshake that failed on the
  stream reads `the TLS handshake failed: kind (endpoint)`. The `ConnectTimeout`
  text of a dial names the endpoint (`no connection to endpoint within 200ms`);
  the other texts (`HeadersTimeout`, `TotalTimeout`, `Interrupted`, the
  `Connect` of three used attempts and every `BodyError`) do not name it. Texts
  keep ASCII only. An idle connection is closed under a claim: a connection that
  is being taken is never closed by its idle timer, and an expired one is never
  taken.
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
  over TLS a body read until close ends cleanly only with a close_notify.
  `Connect` also covers a connection that could not carry the request when no
  byte of it had been written;
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

- `Endpoint`: `https`, `plain_http`, `join_segment`, `with_query_pair`; `Clone`,
  `Eq`, `Debug`, `Display`.
- `Secret`: `new`, `expose`; `Clone`, `Debug`, `Display`.
- `Request`: `new`, `with_header`; `Clone`, `Debug`.
- `HttpConfig`: `new`, `with_total_timeout`, `with_user_agent`,
  `with_extra_root_certificates`; `Clone`, `Eq`, `Debug`.
- `HttpClient`: `new`, `send`; `Clone`, `Debug`. `send` is async and must run
  inside a tokio runtime: called outside one it panics.
- `Response`: `status`, `headers`, `retry_after`, `body_mut`, `into_parts`;
  `Debug`. `Headers`: `get`, `get_all`, `get_text`, `iter`; `Clone`, `Debug`.
  `BodyStream`: `next_chunk`, `poll_next_chunk`, `collect`; `Debug`.
- `Collected`, with the public fields `bytes` and `cut` (`#[non_exhaustive]`;
  `Clone`, `Eq`, `Debug`).
- `Authorizer` and `BoxFuture`; `AuthorizationRequest`: `method`,
  `path_and_query`, `host`, `headers`, `header`, `body`; `Debug`.
  `AuthorizedHeader`, with the public fields `name` and `value`, and `new`
  (`#[non_exhaustive]`; `Clone`, `Debug`).
- `ApiKeyAuthorizer` and `BearerAuthorizer` (`new`; `Clone`, `Debug`);
  `NoAuthorizer` (`Default`, `Copy`, `Clone`, `Debug`).
- `ConfigError`, `SendError`, `BodyError` and `AuthorizerError`
  (`unavailable`, `fetch_failed`): all `#[non_exhaustive]`, `Clone`, `Eq`,
  `Debug`, `Display` and `std::error::Error`. `SendError::Authorizer` carries
  the `AuthorizerError`, which is also the source of that case, and
  `SendError` implements `From<AuthorizerError>`.
