#!/usr/bin/env bash
set -euo pipefail

crate="${1:?usage: release-crate.sh <crate>}"
version=$(bash "$(dirname "$0")/crate-version.sh" "$crate")
changelog="crates/${crate}/CHANGELOG.md"

if [ "$version" = "0.0.0" ]; then
  echo "✓ ${crate} 0.0.0 scaffold — unreleased, nothing to tag"
  exit 0
fi
tag="${crate}-v${version}"

if git rev-parse --verify "refs/tags/${tag}" >/dev/null 2>&1; then
  echo "✓ tag ${tag} already exists"
else
  echo "→ creating tag ${tag}"
  git tag -a "${tag}" \
    -m "${crate} ${version}" \
    -m "Auto-tagged from main. See ${changelog} for the entry."
  git push origin "${tag}"
fi

if gh release view "${tag}" >/dev/null 2>&1; then
  echo "✓ release ${tag} already exists"
else
  echo "→ creating release ${tag}"
  notes_file=$(mktemp)
  awk -v target="## ${version}" '
    !in_section && substr($0, 1, length(target)) == target &&
    (length($0) == length(target) || substr($0, length(target) + 1, 1) == " ") {
      in_section = 1; print; next
    }
    in_section && /^## / { exit }
    in_section { print }
  ' "$changelog" > "$notes_file"
  if [ ! -s "$notes_file" ]; then
    echo "::warning file=${changelog}::could not extract notes for ${tag}; using placeholder"
    printf 'See [%s](https://github.com/%s/blob/%s/%s) for changes.\n' \
      "$changelog" "$GITHUB_REPOSITORY" "$tag" "$changelog" > "$notes_file"
  fi
  gh release create "${tag}" \
    --title "${crate} ${version}" \
    --notes-file "$notes_file" \
    --verify-tag
  rm -f "$notes_file"
fi
