#!/usr/bin/env bash
set -euo pipefail

crate="${1:?usage: crate-version.sh <crate>}"
manifest="crates/${crate}/Cargo.toml"

if [ ! -f "$manifest" ]; then
  echo "::error::${manifest} is missing" >&2
  exit 1
fi

version=$(grep -m1 '^version = ' "$manifest" | sed -E 's/^version = "([^"]+)".*/\1/')
if [ -z "$version" ]; then
  echo "::error file=${manifest}::could not extract the package version" >&2
  exit 1
fi

echo "$version"
