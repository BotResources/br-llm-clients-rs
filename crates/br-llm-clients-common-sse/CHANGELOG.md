# Changelog

All notable changes to `br-llm-clients-common-sse` are documented here. The git tag
`br-llm-clients-common-sse/v{version}` releases the crate; this crate is versioned
separately from the other crates of the repository. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

## 0.1.0

### Added

- `Decoder`, a plain value that turns the bytes of a Server-Sent Events response
  body into frames. `Decoder::new(limit)` refuses a limit of zero with
  `DecodeError::InvalidLimit`; `Decoder::default()` uses `DEFAULT_LIMIT`
  (8 MiB, 8 388 608 bytes). `feed(chunk, &mut frames)` appends the frames the
  chunk completes, in order, to a list the caller owns, so frames completed
  before an error in the same chunk are not lost. `finish()` tells the decoder
  that the input has ended.
- `Frame`, with `event()` (absent when empty), `data()` (data lines joined with
  one line feed, none at the end), `into_parts()` and `Frame::new`.
- `DecodeError`, one error type owned by the library, non-exhaustive, with a text
  for people on every case: `InvalidLimit`, `FrameTooLarge { limit }`,
  `InvalidUtf8`, `InputTruncated`, `InputEnded`. No case carries a lower layer's
  error.
- The frames returned do not depend on the chunk boundaries. A frame is returned
  by the chunk that holds its closing line end; a carriage return is a complete
  line end without waiting for a line feed. A leading byte order mark is skipped.
- The held bytes (unfinished line, event name and data text of the open frame)
  are limited; complete comment, `id`, `retry` and unknown field lines hold
  nothing.
- After an error, `feed` and `finish` return the same error again. After
  `finish`, `feed` returns `InputEnded`.
- Departures from the WHATWG rules, on purpose: invalid UTF-8 in a `data` or
  `event` line is an error, an input that ends inside a frame is an error, and a
  size limit exists.
- No dependency, no runtime, no network, no clock.
