# br-llm-clients

Provider clients for LLM calls: send a system prompt, tools and a chat history from `br-llm-messages`, get the reply. Provider clients are separate crates, versioned separately, each released by its own git tag `{crate}/v{version}`:

- `br-llm-clients-common-http` — sends one HTTP POST and gives back the status, the headers and a stream of byte chunks, with every failure told apart; it is used by the provider clients `br-llm-clients-openai`, `br-llm-clients-anthropic`, `br-llm-clients-litellm`, `br-llm-clients-openrouter`, `br-llm-providers-omlx` and `br-llm-providers-mlx-vlm`.
- `br-llm-clients-core` — the core: provider-neutral request over a projected thread, settings, outcome, error, the rendering of a history, and the `Client`, `StreamSink` and `IdSource` traits every client library uses.
- `br-llm-clients-mock` — a configurable mock `Client` for fast, parallel tests, used as a dev-dependency.
- `br-llm-clients-common-call` — runs one call of a client: encodes the request through a dialect, sends it, and turns every failure into a `ClientError`, for any provider and any wire dialect; it is used by the provider clients `br-llm-clients-openai`, `br-llm-clients-anthropic`, `br-llm-clients-litellm`, `br-llm-clients-openrouter`, `br-llm-providers-omlx` and `br-llm-providers-mlx-vlm`.
- `br-llm-clients-api-anthropic-messages` — the Anthropic Messages wire dialect: encodes a request and decodes the streamed reply and the errors of the Anthropic Messages API for the shared call; it is used by the provider clients `br-llm-clients-anthropic`, `br-llm-clients-litellm` and `br-llm-clients-openrouter`.
- `br-llm-clients-api-openai-responses` — the OpenAI Responses wire dialect: encodes a request and decodes the streamed reply and the errors of the OpenAI Responses API for the shared call; it is used by the provider clients `br-llm-clients-openai`, `br-llm-clients-litellm` and `br-llm-clients-openrouter`.
- `br-llm-clients-api-openai-chat-completions` — the OpenAI Chat Completions wire dialect: encodes a request and decodes the streamed reply and the errors of the Chat Completions API for the shared call; it is used by the provider clients `br-llm-clients-litellm`, `br-llm-clients-openrouter`, `br-llm-providers-omlx` and `br-llm-providers-mlx-vlm`.
- `br-llm-clients-openai` — the client for the first-party OpenAI API: sends every call to the Responses API with an API key, checks the capabilities of its model profile before each call, and gives the outcome of the shared call. Its optional base is the origin, with an optional path prefix, before `/v1`: the client adds `/v1/responses` itself.
- `br-llm-clients-anthropic` — the client for the first-party Anthropic API: sends every call to the Messages API, with the key in `x-api-key` or, on request, in `authorization: Bearer`, checks the capabilities of the current Claude models before each call, and gives the outcome of the shared call. Its optional base is the origin, with an optional path prefix, before `/v1`: the client adds `/v1/messages` itself.
- `br-llm-clients-common-sse` — a Server-Sent Events decoder: turns the bytes of a response body into frames, with a size limit and no hidden damage.
- `br-llm-clients-common-auth` — gives a bearer token to a request: a token source, a cache that fetches a new token before the old one ends, an authorizer for `br-llm-clients-common-http` and a client for the form-encoded token request of OAuth 2.0; no provider client uses it yet.
- `br-llm-clients-litellm` — the `Client` for a LiteLLM proxy: sends a call to the proxy in the chat completions, responses or messages dialect, with a virtual key and the capabilities the operator declares for a name.
- `br-llm-clients-openrouter` — the `Client` for OpenRouter: sends a call to OpenRouter in the chat completions, responses or messages dialect, with an API key, optional app attribution and the capabilities the operator declares for a model.
- `br-llm-providers-omlx` — the `Client` for one oMLX server: sends a call to its chat route, with an optional key and the capabilities the operator declares for the model.
- `br-llm-providers-mlx-vlm` — the `Client` for one mlx-vlm server: sends a call to its chat route, with an optional key and the capabilities the operator declares for the model.

**Status:** `br-llm-clients-core` is at `0.5.0`, `br-llm-clients-mock` is at `0.6.0`, `br-llm-clients-common-http`, `br-llm-clients-common-call`, `br-llm-clients-api-anthropic-messages`, `br-llm-clients-api-openai-responses` and `br-llm-clients-api-openai-chat-completions` are at `0.2.0`, and `br-llm-clients-common-sse`, `br-llm-clients-common-auth`, `br-llm-clients-openai`, `br-llm-clients-anthropic`, `br-llm-clients-litellm`, `br-llm-clients-openrouter`, `br-llm-providers-omlx` and `br-llm-providers-mlx-vlm` are at `0.1.0`; the other crates are not written yet.

Not on crates.io; pin the git tag with the matching `version` beside it (a tag-only git dependency carries a `*` version requirement, which `cargo-deny` rejects):

```toml
br-llm-clients-core = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-core", tag = "br-llm-clients-core/v0.5.0", version = "0.5.0" }
br-llm-clients-mock = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-mock", tag = "br-llm-clients-mock/v0.6.0", version = "0.6.0" }
br-llm-clients-common-sse = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-sse", tag = "br-llm-clients-common-sse/v0.1.0", version = "0.1.0" }
br-llm-clients-common-call = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-call", tag = "br-llm-clients-common-call/v0.2.0", version = "0.2.0" }
br-llm-clients-api-anthropic-messages = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-anthropic-messages", tag = "br-llm-clients-api-anthropic-messages/v0.2.0", version = "0.2.0" }
br-llm-clients-api-openai-responses = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-responses", tag = "br-llm-clients-api-openai-responses/v0.2.0", version = "0.2.0" }
br-llm-clients-api-openai-chat-completions = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-api-openai-chat-completions", tag = "br-llm-clients-api-openai-chat-completions/v0.2.0", version = "0.2.0" }
br-llm-clients-common-http = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-http", tag = "br-llm-clients-common-http/v0.2.0", version = "0.2.0" }
br-llm-clients-common-auth = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-common-auth", tag = "br-llm-clients-common-auth/v0.1.0", version = "0.1.0" }
br-llm-clients-openai = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-openai", tag = "br-llm-clients-openai/v0.1.0", version = "0.1.0" }
br-llm-clients-anthropic = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-anthropic", tag = "br-llm-clients-anthropic/v0.1.0", version = "0.1.0" }
br-llm-clients-litellm = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-litellm", tag = "br-llm-clients-litellm/v0.1.0", version = "0.1.0" }
br-llm-clients-openrouter = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-clients-openrouter", tag = "br-llm-clients-openrouter/v0.1.0", version = "0.1.0" }
br-llm-providers-omlx = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-providers-omlx", tag = "br-llm-providers-omlx/v0.1.0", version = "0.1.0" }
br-llm-providers-mlx-vlm = { git = "https://github.com/BotResources/br-llm-clients-rs", package = "br-llm-providers-mlx-vlm", tag = "br-llm-providers-mlx-vlm/v0.1.0", version = "0.1.0" }
```

`br-llm-clients-mock` (0.6.0) pins `br-llm-clients-core` at `br-llm-clients-core/v0.5.0` and `br-llm-messages` at `br-llm-messages/v2.1.0`; a project that uses the mock and names core or messages pins those same two tags, or the types are two copies.

`br-llm-clients-common-call` (0.2.0) pins `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0`, `br-llm-clients-common-sse` at `br-llm-clients-common-sse/v0.1.0` and `br-llm-messages` at `br-llm-messages/v2.1.0`, and it re-exports `Authorizer`, `Endpoint`, `Headers`, `HttpClient` and `Secret` of `br-llm-clients-common-http`; a project that uses common-call and names a type of these crates pins those same tags, or the types are two copies.

`br-llm-clients-api-openai-chat-completions` (0.2.0) pins `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-core` at `br-llm-clients-core/v0.5.0` and `br-llm-messages` at `br-llm-messages/v2.1.0`, and it re-exports the types of common-call, core and messages that a provider library needs to build and wrap the dialect, `Headers` of `br-llm-clients-common-http` 0.2 included; a project that uses it pins common-call, core and messages at those same tags, or the types are two copies.

`br-llm-clients-api-openai-responses` (0.2.0) pins `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-core` at `br-llm-clients-core/v0.5.0` and `br-llm-messages` at `br-llm-messages/v2.1.0`; a project that uses it pins common-call, core and messages at those same tags, or the types are two copies.

`br-llm-clients-api-anthropic-messages` (0.2.0) pins `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-core` at `br-llm-clients-core/v0.5.0` and `br-llm-messages` at `br-llm-messages/v2.1.0`; a project that uses it pins common-call, core and messages at those same tags, or the types are two copies.

`br-llm-clients-openai` (0.1.0) pins `br-llm-clients-api-openai-responses` at `br-llm-clients-api-openai-responses/v0.2.0`, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`, and uses `br-llm-messages` 2.1.0 through them (a `ClientSettings` needs a `ModelName`); a project that uses it and names a type of these crates pins them at those same tags, and `br-llm-messages` at `br-llm-messages/v2.1.0` with `version = "2.1.0"`, or the types are two copies.

`br-llm-clients-anthropic` (0.1.0) pins `br-llm-clients-api-anthropic-messages` at `br-llm-clients-api-anthropic-messages/v0.2.0`, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`, and uses `br-llm-messages` 2.1.0 through them; no type of common-call or of the dialect is in its public API. A project that uses it and names a type of core, common-http or messages (`ClientSettings`, `ClientRequest`, `Secret`, `Endpoint`, `HttpClient`, `PlainText`, `TurnId`, `EntryId`, `ModelResponse`) pins them at those same tags, and `br-llm-messages` at `br-llm-messages/v2.1.0` with `version = "2.1.0"`, or the types are two copies.

`br-llm-clients-common-auth` (0.1.0) uses `br-llm-clients-common-http` 0.2.0 in its public API (`Secret`, `Authorizer`, `AuthorizerError`, `BoxFuture`, `HttpClient`, `Endpoint`); a project that uses it pins `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0`, or the types are two copies. `br-llm-clients-common-call` 0.2.0 uses the same pin, so a `TokenAuthorizer` can be given to it.

`br-llm-clients-litellm` (0.1.0) pins `br-llm-clients-api-openai-chat-completions`, `br-llm-clients-api-openai-responses` and `br-llm-clients-api-anthropic-messages` at their `v0.2.0` tags, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`. It re-exports the types needed to build a `LiteLlmConfig`: `Endpoint`, `Secret`, the `IdSource`, the options types of the three dialects and the capabilities types of the core. A project builds the `ClientSettings` with `br-llm-clients-core` and the `HttpClient` (with its `HttpConfig`) with `br-llm-clients-common-http`, so it pins those two at the same tags, and names a type of common-call or of an api library at that library's tag, or the types are two copies.

`br-llm-clients-openrouter` (0.1.0) pins `br-llm-clients-api-openai-chat-completions`, `br-llm-clients-api-openai-responses` and `br-llm-clients-api-anthropic-messages` at their `v0.2.0` tags, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`. It re-exports the types needed to build an `OpenRouterConfig`: `Endpoint`, `Secret`, the `IdSource`, the options types of the three dialects and the capabilities types of the core. A project builds the `ClientSettings` with `br-llm-clients-core` and the `HttpClient` (with its `HttpConfig`) with `br-llm-clients-common-http`, so it pins those two at the same tags, and names a type of common-call or of an api library at that library's tag, or the types are two copies.

`br-llm-providers-omlx` (0.1.0) pins `br-llm-clients-api-openai-chat-completions` at `br-llm-clients-api-openai-chat-completions/v0.2.0`, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`. It re-exports the types needed to build an `OmlxConfig`: `Endpoint`, `Secret`, `HttpClient`, the `IdSource`, `ReasoningTable` with `ReasoningRule` and `ReasoningEffort`, and the capabilities types of the core. A project builds the `ClientSettings` with `br-llm-clients-core` and the `HttpConfig` with `br-llm-clients-common-http`, so it pins those two at the same tags, or the types are two copies.

`br-llm-providers-mlx-vlm` (0.1.0) pins `br-llm-clients-api-openai-chat-completions` at `br-llm-clients-api-openai-chat-completions/v0.2.0`, `br-llm-clients-common-call` at `br-llm-clients-common-call/v0.2.0`, `br-llm-clients-common-http` at `br-llm-clients-common-http/v0.2.0` and `br-llm-clients-core` at `br-llm-clients-core/v0.5.0`. It re-exports the types needed to build an `MlxVlmConfig`: `Endpoint`, `Secret`, `HttpClient`, the `IdSource`, `ReasoningTable` with `ReasoningRule` and `ReasoningEffort`, and the capabilities types of the core. A project builds the `ClientSettings` with `br-llm-clients-core` and the `HttpConfig` with `br-llm-clients-common-http`, so it pins those two at the same tags, or the types are two copies.

## Provider keys for real calls

Tests never call a provider. To record a real stream (for example a fixture), copy `.env.template` to `.env`, which git ignores, and fill in the keys you need:

```bash
cp .env.template .env
```

Nothing in this repository reads `.env` by itself; a script that records a call loads it.

The live check of `br-llm-clients-openai` (`cargo run -p br-llm-clients-openai --example live_check`) reads only the process environment: `OPENAI_API_KEY` is required, `OPENAI_BASE_URL` and `OPENAI_MODEL` are optional. Export them before you run it.

The live check of `br-llm-clients-anthropic` (`cargo run -p br-llm-clients-anthropic --example live_check`) reads only the process environment: `ANTHROPIC_API_KEY` and `ANTHROPIC_BASE_URL` are required, `ANTHROPIC_MODEL`, `ANTHROPIC_WORKSPACE_ID` and `ANTHROPIC_AUTH` (`x-api-key` or `bearer`) are optional. Export them before you run it.

The live check of `br-llm-providers-omlx` (`cargo run -p br-llm-providers-omlx --example live_check`) and its example `prefix_cache` read only the process environment: `OMLX_BASE_URL` (the root of the server, without `/v1`) and `OMLX_MODEL` are required; `OMLX_API_KEY`, `OMLX_STRUCTURED` (`1` for a server installed with `omlx[grammar]`) and `OMLX_MODEL_FALLBACK` (`1` when the model fallback setting of the server is on) are optional. Export them before you run it.

The live check of `br-llm-providers-mlx-vlm` (`cargo run -p br-llm-providers-mlx-vlm --example live_check`) and its example `prefix_cache` read only the process environment: `MLX_VLM_BASE_URL` (the root of the server, without `/v1`) and `MLX_VLM_MODEL` are required; `MLX_VLM_API_KEY` is optional. Its prefix cache check needs a server started with `APC_ENABLED=1`. Export them before you run it.

No support is provided; paid support is available on request from BotResources. Apache-2.0. Generated by an AI system (Anthropic Claude) under the direction and review of BotResources, which takes full responsibility for it (EU AI Act, Regulation (EU) 2024/1689).
