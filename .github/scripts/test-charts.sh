#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

DIALECT=charts/br-llm-clients-api-anthropic-messages-chart

failures=0

fail() {
  echo "::error::$1" >&2
  failures=$((failures + 1))
}

render() {
  helm template example "$@" 2>&1
}

refuses() {
  local name="$1"
  shift
  if helm template example "$@" >/dev/null 2>&1; then
    fail "${name}: the render passed with $*"
  else
    echo "✓ ${name}: refused $*"
  fi
}

holds() {
  local name="$1" rendered="$2" line="$3"
  if grep -qxF -- "$line" <<<"$rendered"; then
    echo "✓ ${name}: ${line# *}"
  else
    echo "$rendered" >&2
    fail "${name}: the render does not hold the line: ${line}"
  fi
}

lacks() {
  local name="$1" rendered="$2" text="$3"
  if grep -qF -- "$text" <<<"$rendered"; then
    echo "$rendered" >&2
    fail "${name}: the render holds ${text}"
  else
    echo "✓ ${name}: no ${text}"
  fi
}

crate_variable() {
  sed -nE 's/^pub const CACHE_ENV: &str = "([A-Z_]+)";$/\1/p' "crates/$1/src/cache.rs"
}

fixture_alias() {
  sed -nE 's/^[[:space:]]*alias:[[:space:]]*"?([a-z0-9_]+)"?[[:space:]]*$/\1/p' "$1/Chart.yaml" | head -n1
}

helm lint --strict "$DIALECT"
helm dependency build "$DIALECT/ci/example" >/dev/null
helm lint --strict "$DIALECT/ci/example"
example="$DIALECT/ci/example"
holds dialect "$(render "$example")" "  cache: '{\"ttl\":\"1h\"}'"
holds dialect "$(render "$example" --set messages.ttl=5m)" "  cache: '{\"ttl\":\"5m\"}'"
holds dialect "$(render "$example" --set messages=null)" "  cache: '{}'"
holds dialect "$(render "$example" --set global.team=ocelot)" "  cache: '{\"ttl\":\"1h\"}'"
refuses dialect "$example" --set messages.ttl=2h
refuses dialect "$example" --set messages.ttl=1H
refuses dialect "$example" --set messages.ttl=1
refuses dialect "$example" --set messages.ttl=null
refuses dialect "$example" --set messages.lifetime=1h

for env_template in charts/*/templates/_env.tpl; do
  [ -f "$env_template" ] || continue
  chart="${env_template%/templates/_env.tpl}"
  crate="${chart#charts/}"
  crate="${crate%-chart}"
  example="${chart}/ci/example"
  alias=$(fixture_alias "$example")
  if [ -z "$alias" ]; then
    fail "${crate}: ${example}/Chart.yaml declares no alias for the chart"
    continue
  fi
  variable=$(crate_variable "$crate")
  if [ -z "$variable" ] || ! grep -qF -- "- name: ${variable}" "${chart}/templates/_env.tpl"; then
    fail "${crate}: the chart does not render the variable CACHE_ENV of the crate (${variable:-none})"
  fi
  helm dependency build "$chart" >/dev/null
  helm lint --strict "$chart"
  helm dependency build "$example" >/dev/null
  helm lint --strict "$example"
  rendered=$(render "$example")
  holds "$crate" "$rendered" "            - name: ${variable}"
  holds "$crate" "$rendered" "              value: '{\"messages\":{\"ttl\":\"1h\"}}'"
  holds "$crate" "$(render "$example" --set "${alias}.messages.ttl=5m")" \
    "              value: '{\"messages\":{\"ttl\":\"5m\"}}'"
  holds "$crate" "$(render "$example" --set "global.team=ocelot")" \
    "              value: '{\"messages\":{\"ttl\":\"1h\"}}'"
  unset_render=$(render "$example" --set "${alias}.messages=null")
  holds "$crate" "$unset_render" "            - name: RUNNER_NAME"
  lacks "$crate" "$unset_render" "$variable"
  refuses "$crate" "$example" --set "${alias}.messages.ttl=2h"
  refuses "$crate" "$example" --set "${alias}.messages.lifetime=1h"
done

if [ "$failures" -gt 0 ]; then
  echo "${failures} chart check(s) failed" >&2
  exit 1
fi
echo "all chart checks passed"
