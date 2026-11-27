{{- define "br-llm-clients-aws-bedrock-chart.env" -}}
{{- $values := default dict . -}}
{{- $messages := omit (default dict $values.messages) "global" -}}
{{- if $messages -}}
- name: AWS_BEDROCK_CACHE
  value: {{ printf "{\"messages\":%s}" (include "br-llm-clients-api-anthropic-messages-chart.cache" $messages) | squote }}
{{- end -}}
{{- end -}}
