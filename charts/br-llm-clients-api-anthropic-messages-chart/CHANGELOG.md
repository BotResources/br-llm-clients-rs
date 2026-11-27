# Changelog

## 0.1.0

First release. A library chart with one named template, `br-llm-clients-api-anthropic-messages-chart.cache`, that renders the cache options of the Anthropic Messages dialect as a JSON object, for the setting of a provider library. It is compatible with `br-llm-clients-api-anthropic-messages` 0.3.

- The values are those of `MessagesCacheOptions`: `ttl`, optional, `5m` or `1h`. With no `ttl` the template renders `{}`, and the library gives its default, five minutes.
- `values.schema.json` accepts what `MessagesCacheOptions::from_json` accepts, plus the member `global` that Helm adds. A `ttl` that is not `5m` or `1h` (a `null` included) and any other member fail the render. The template refuses the same values when it is given a map that the schema did not check.
- It defines no environment variable. A provider chart depends on it under the alias `messages` and puts the object in its own setting.
