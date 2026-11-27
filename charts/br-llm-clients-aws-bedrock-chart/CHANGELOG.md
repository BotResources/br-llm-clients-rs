# Changelog

## 0.1.0

First release. A library chart with one named template, `br-llm-clients-aws-bedrock-chart.env`, that renders the cache setting of `br-llm-clients-aws-bedrock` as the environment variable `AWS_BEDROCK_CACHE` of the container of a runner. It is compatible with `br-llm-clients-aws-bedrock` 0.3, the first line that names `AWS_BEDROCK_CACHE`, and it holds `br-llm-clients-api-anthropic-messages-chart` 0.1.0.

- The chart depends on `br-llm-clients-api-anthropic-messages-chart` 0.1.0 under the alias `messages`. The values under `messages` follow the schema of that chart: `ttl`, optional, `5m` or `1h`.
- The template renders one entry, `AWS_BEDROCK_CACHE`, whose text is `{"messages": …}` with the object that the dialect template renders. With no value under `messages`, it renders nothing, and the library gives its default, five minutes.
