# br-llm-clients

Provider clients for LLM calls: send a system prompt, tools and a chat history from `br-llm-messages`, get the reply. Provider clients are separate crates, versioned separately, each released by its own git tag `{crate}/v{version}`:

- `br-llm-clients-common-http` — sends one HTTP POST and gives back the status, the headers and a stream of byte chunks, with every failure told apart; it is meant to be shared by the provider clients (none exists yet).
- `br-llm-clients-core` — the core: provider-neutral request over a projected thread, settings, outcome, error, the rendering of a history, and the `Client`, `StreamSink` and `IdSource` traits every client library uses.
- `br-llm-clients-mock` — a configurable mock `Client` for fast, parallel tests, used as a dev-dependency.
- `br-llm-clients-common-call` — runs one call of a client: encodes the request through a dialect, sends it, and turns every failure into a `ClientError`, for any provider and any wire dialect; it is meant to be shared by the provider clients (none exists yet).
- `br-llm-clients-api-anthropic-messages` — the Anthropic Messages wire dialect: encodes a request and decodes the streamed reply and the errors of the Anthropic Messages API for the shared call; it is meant to be used by the provider clients (none exists yet).
- `br-llm-clients-api-openai-responses` — the OpenAI Responses wire dialect: encodes a request and decodes the streamed reply and the errors of the OpenAI Responses API for the shared call; it is meant to be used by the provider clients (none exists yet).
- `br-llm-clients-api-openai-chat-completions` — the OpenAI Chat Completions wire dialect: encodes a request and decodes the streamed reply and the errors of the Chat Completions API for the shared call; it is meant to be used by the provider clients (none exists yet).
- `br-llm-clients-common-sse` — a Server-Sent Events decoder: turns the bytes of a response body into frames, with a size limit and no hidden damage.

**Status:** `br-llm-clients-core` is at `0.5.0`, `br-llm-clients-mock` is at `0.5.0`, `br-llm-clients-common-http` is at `0.2.0`, and `br-llm-clients-common-sse`, `br-llm-clients-common-call`, `br-llm-clients-api-anthropic-messages`, `br-llm-clients-api-openai-responses` and `br-llm-clients-api-openai-chat-completions` are at `0.1.0`; the other crates are not written yet.

Not on crates.io; pin the git tag with the matching `version` beside it (a tag-only git dependency carries a `*` version requirement, which `cargo-deny` rejects):

```toml
br-llm-clients-core = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-core", tag = "br-llm-clients-core/v0.5.0", version = "0.5.0" }
br-llm-clients-mock = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-mock", tag = "br-llm-clients-mock/v0.5.0", version = "0.5.0" }
br-llm-clients-common-sse = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-sse", tag = "br-llm-clients-common-sse/v0.1.0", version = "0.1.0" }
br-llm-clients-common-call = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-call", tag = "br-llm-clients-common-call/v0.1.0", version = "0.1.0" }
br-llm-clients-api-anthropic-messages = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-anthropic-messages", tag = "br-llm-clients-api-anthropic-messages/v0.1.0", version = "0.1.0" }
br-llm-clients-api-openai-responses = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-responses", tag = "br-llm-clients-api-openai-responses/v0.1.0", version = "0.1.0" }
br-llm-clients-api-openai-chat-completions = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-chat-completions", tag = "br-llm-clients-api-openai-chat-completions/v0.1.0", version = "0.1.0" }
br-llm-clients-common-http = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-http", tag = "br-llm-clients-common-http/v0.2.0", version = "0.2.0" }
```

`br-llm-clients-common-call` (0.1.0) re-exports types of `br-llm-clients-common-http` 0.1.0 (`Authorizer`, `Endpoint`, `Headers`, `HttpClient`, `Secret`), so a project that uses it pins `br-llm-clients-common-http` at `v0.1.0` too until `br-llm-clients-common-call` moves to 0.2.0: with the 0.2.0 pin shown, those types would be two copies. The three `br-llm-clients-api-*` libraries (0.1.0) use `br-llm-clients-common-http` 0.1.0 only in their tests and do not re-export it.

## Provider keys for real calls

Tests never call a provider. To record a real stream (for example a fixture), copy `.env.template` to `.env`, which git ignores, and fill in the keys you need:

```bash
cp .env.template .env
```

Nothing in this repository reads `.env` by itself; a script that records a call loads it.

No support is provided; paid support is available on request from BotResources. Apache-2.0. Generated by an AI system (Anthropic Claude) under the direction and review of BotResources, which takes full responsibility for it (EU AI Act, Regulation (EU) 2024/1689).
