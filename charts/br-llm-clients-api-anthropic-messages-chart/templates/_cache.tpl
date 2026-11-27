{{- define "br-llm-clients-api-anthropic-messages-chart.cache" -}}
{{- $values := default dict . -}}
{{- range $member, $_ := omit $values "ttl" "global" -}}
{{- fail (printf "br-llm-clients-api-anthropic-messages-chart: the member %s is not a cache option" $member) -}}
{{- end -}}
{{- $cache := pick $values "ttl" -}}
{{- if and (hasKey $cache "ttl") (not (has $cache.ttl (list "5m" "1h"))) -}}
{{- fail "br-llm-clients-api-anthropic-messages-chart: ttl must be 5m or 1h" -}}
{{- end -}}
{{- toJson $cache -}}
{{- end -}}
