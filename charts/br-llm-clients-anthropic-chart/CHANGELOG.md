# Changelog

## 0.1.0

First release. A library chart with one named template, `br-llm-clients-anthropic-chart.env`, that renders the cache setting of `br-llm-clients-anthropic` as the environment variable `ANTHROPIC_CACHE` of the container of a runner. It is compatible with `br-llm-clients-anthropic` 0.2, the first line that names `ANTHROPIC_CACHE`, and it holds `br-llm-clients-api-anthropic-messages-chart` 0.1.0.

- The chart depends on `br-llm-clients-api-anthropic-messages-chart` 0.1.0 under the alias `messages`. The values under `messages` follow the schema of that chart: `ttl`, optional, `5m` or `1h`.
- The template renders one entry, `ANTHROPIC_CACHE`, whose text is `{"messages": …}` with the object that the dialect template renders. With no value under `messages`, it renders nothing, and the library gives its default, five minutes.
