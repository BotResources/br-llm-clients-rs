# br-llm-clients

Provider clients for LLM calls: send a system prompt, tools and a chat history from `br-llm-messages`, get the reply. Provider clients are separate crates, versioned separately, each released by its own git tag `{crate}/v{version}`:

- `br-llm-clients-common-http` — sends one HTTP POST and gives back the status, the headers and a stream of byte chunks, with every failure told apart; it is shared by the provider clients.
- `br-llm-clients-core` — the core: provider-neutral request, settings, outcome, error and the `Client` and `StreamSink` traits every client library implements.
- `br-llm-clients-mock` — a configurable mock `Client` for fast, parallel tests, used as a dev-dependency.
- `br-llm-clients-common-call` — runs one call of a client: encodes the request through a dialect, sends it, and turns every failure into a `ClientError`, for any provider and any wire dialect; it is shared by the provider clients.
- `br-llm-clients-api-anthropic-messages` — the Anthropic Messages wire dialect: encodes a request and decodes the streamed reply and the errors of the Anthropic Messages API for the shared call; `br-llm-clients-anthropic` and `br-llm-clients-litellm` use it.
- `br-llm-clients-api-openai-responses` — the OpenAI Responses wire dialect: encodes a request and decodes the streamed reply and the errors of the OpenAI Responses API for the shared call; the provider clients use it.
- `br-llm-clients-api-openai-chat-completions` — the OpenAI Chat Completions wire dialect: encodes a request and decodes the streamed reply and the errors of the Chat Completions API for the shared call; the provider clients use it.
- `br-llm-clients-kimi` — the client of the Kimi models of Moonshot AI: a `Client` of `br-llm-clients-core` over the Chat Completions API of a Moonshot host, with the way each Kimi model takes thinking.
- `br-llm-clients-anthropic` — the `Client` of the Anthropic first-party API (the Messages API on `api.anthropic.com`, streaming): where to send, how to authorize, and the declared capabilities of the current Claude models.
- `br-llm-clients-openai` — a `Client` for the first-party OpenAI API: sends every call to the Responses API with an API key, and knows what each model preset accepts.
- `br-llm-clients-common-sse` — a Server-Sent Events decoder: turns the bytes of a response body into frames, with a size limit and no hidden damage.
- `br-llm-clients-litellm` — the `Client` for a LiteLLM proxy: sends a call to the proxy in the chat completions, responses or messages dialect, with a virtual key and the capabilities the operator declares for a name.

**Status:** `br-llm-clients-core` is at `0.4.1`, `br-llm-clients-mock` is at `0.5.0`, and `br-llm-clients-common-sse`, `br-llm-clients-common-http`, `br-llm-clients-common-call`, `br-llm-clients-api-anthropic-messages`, `br-llm-clients-api-openai-responses`, `br-llm-clients-api-openai-chat-completions`, `br-llm-clients-anthropic`, `br-llm-clients-openai`, `br-llm-clients-kimi` and `br-llm-clients-litellm` are at `0.1.0`.

Not on crates.io; pin the git tag with the matching `version` beside it (a tag-only git dependency carries a `*` version requirement, which `cargo-deny` rejects):

```toml
br-llm-clients-core = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-core", tag = "br-llm-clients-core/v0.4.1", version = "0.4.1" }
br-llm-clients-mock = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-mock", tag = "br-llm-clients-mock/v0.5.0", version = "0.5.0" }
br-llm-clients-common-sse = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-sse", tag = "br-llm-clients-common-sse/v0.1.0", version = "0.1.0" }
br-llm-clients-common-call = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-call", tag = "br-llm-clients-common-call/v0.1.0", version = "0.1.0" }
br-llm-clients-api-anthropic-messages = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-anthropic-messages", tag = "br-llm-clients-api-anthropic-messages/v0.1.0", version = "0.1.0" }
br-llm-clients-api-openai-responses = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-responses", tag = "br-llm-clients-api-openai-responses/v0.1.0", version = "0.1.0" }
br-llm-clients-api-openai-chat-completions = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-chat-completions", tag = "br-llm-clients-api-openai-chat-completions/v0.1.0", version = "0.1.0" }
br-llm-clients-anthropic = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-anthropic", tag = "br-llm-clients-anthropic/v0.1.0", version = "0.1.0" }
br-llm-clients-common-http = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-http", tag = "br-llm-clients-common-http/v0.1.0", version = "0.1.0" }
br-llm-clients-kimi = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-kimi", tag = "br-llm-clients-kimi/v0.1.0", version = "0.1.0" }
br-llm-clients-litellm = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-litellm", tag = "br-llm-clients-litellm/v0.1.0", version = "0.1.0" }
br-llm-clients-openai = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-openai", tag = "br-llm-clients-openai/v0.1.0", version = "0.1.0" }
```

## Provider keys for real calls

The example `live_check` of a provider crate calls the real provider, by hand, with a real key. To run it, copy `.env.template` to `.env`, which git ignores, and fill in the keys you need:

```bash
cp .env.template .env
```

No library of this repository reads `.env`; only the `live_check` example of a provider crate loads it.

No support is provided; paid support is available on request from BotResources. Apache-2.0. Generated by an AI system (Anthropic Claude) under the direction and review of BotResources, which takes full responsibility for it (EU AI Act, Regulation (EU) 2024/1689).
