# br-llm-clients-anthropic-chart

A Helm library chart. It holds the cache setting of `br-llm-clients-anthropic`, the Anthropic first-party API client for Claude, and one named template that renders it as the environment variable `ANTHROPIC_CACHE` of the container of a runner. It is a library chart: it renders nothing by itself.

## Use

The chart of a runner declares this chart as a dependency under an alias, sets the values under that alias, and includes the named template in the `env` of its container, with the values of the alias:

```yaml
dependencies:
  - name: br-llm-clients-anthropic-chart
    alias: anthropic
    version: "0.1.0"
    repository: "oci://ghcr.io/botresources/charts"
```

```yaml
anthropic:
  messages:
    ttl: 1h
```

```yaml
env:
  {{- include "br-llm-clients-anthropic-chart.env" .Values.anthropic | nindent 2 }}
```

This renders:

```yaml
env:
  - name: ANTHROPIC_CACHE
    value: '{"messages":{"ttl":"1h"}}'
```

With no value under `messages`, the template renders nothing, and the library gives its default.

## Values

| Value | Rule | Absent |
|---|---|---|
| `messages.ttl` | `5m` or `1h`: the lifetime of the cache that `CacheRequirement::Required` asks for | `5m`, the default of the library |

The values under `messages` are those of `br-llm-clients-api-anthropic-messages-chart`, a dependency of this chart under the alias `messages`. Its `values.schema.json` is checked at the path of the alias: any other member under `messages`, and a `ttl` that is not one of the two texts, fail the render.

The library reads no environment variable. The runner reads `ANTHROPIC_CACHE` when it starts and gives its text to `parse` of the cache type of `br-llm-clients-anthropic`, then gives the result to `with_cache` of its config.

## Why

| Thing | Why it is the way it is |
|---|---|
| The values of the alias given to the template | The template does not depend on the alias that a runner chooses. |
| The JSON object `{"messages": …}` | The setting of the library holds one section per dialect. The section `messages` is parsed by the dialect, so a later dialect option needs no change of this chart beyond the version of the dialect chart. |
| One version of the dialect chart per chart tree | Helm keeps one template per name for the whole tree of a release. A runner chart that depends on several provider charts must use versions of them that hold the same version of `br-llm-clients-api-anthropic-messages-chart`. |
