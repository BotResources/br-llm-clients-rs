# br-llm-clients-api-anthropic-messages-chart

A Helm library chart. It holds the cache options of the Anthropic Messages dialect of `br-llm-clients-api-anthropic-messages`, as an operator writes them, and one named template that renders them as a JSON object. It is a library chart: it renders nothing by itself, and it defines no environment variable.

A runner does not use this chart directly. It uses the chart of its provider library, which depends on this one under the alias `messages`:

| Provider chart | Environment variable |
|---|---|
| `br-llm-clients-anthropic-chart` | `ANTHROPIC_CACHE` |
| `br-llm-clients-aws-bedrock-chart` | `AWS_BEDROCK_CACHE` |
| `br-llm-clients-gcp-vertex-chart` | `GCP_VERTEX_CACHE` |

## Values

| Value | Rule | Absent |
|---|---|---|
| `ttl` | `5m` or `1h`: the lifetime of the cache that `CacheRequirement::Required` asks for | `5m`, the default of the library |

`values.schema.json` accepts exactly what `MessagesCacheOptions::from_json` of the library accepts, plus the member `global` that Helm adds to the values of every chart. Any other member, and a `ttl` that is not one of the two texts, fail the render.

## Template

`br-llm-clients-api-anthropic-messages-chart.cache` takes the values of this chart and renders their JSON object, without `global`:

```yaml
messages:
  ttl: 1h
```

```text
{{ include "br-llm-clients-api-anthropic-messages-chart.cache" .Values.messages }}
```

renders `{"ttl":"1h"}`. With no `ttl` it renders `{}`.

## One version per chart tree

Helm keeps one template per name for the whole tree of a release. A runner chart that depends on two provider charts must use versions of them that hold the same version of this chart, or one of the two templates is silently replaced by the other.
