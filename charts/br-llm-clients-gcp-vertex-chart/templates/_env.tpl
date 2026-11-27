{{- define "br-llm-clients-gcp-vertex-chart.env" -}}
{{- $values := default dict . -}}
{{- $messages := omit (default dict $values.messages) "global" -}}
{{- if $messages -}}
- name: GCP_VERTEX_CACHE
  value: {{ printf "{\"messages\":%s}" (include "br-llm-clients-api-anthropic-messages-chart.cache" $messages) | squote }}
{{- end -}}
{{- end -}}
