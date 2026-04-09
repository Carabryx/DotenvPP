#!/usr/bin/env bash

set -euo pipefail

tag_ref="${1:-${GITHUB_REF:-}}"

if [[ -z "$tag_ref" ]]; then
  echo "❌ GITHUB_REF must be set or passed as the first argument"
  exit 1
fi

package_version="$(
  awk '
    /^\[workspace\.package\]$/ { in_section = 1; next }
    /^\[/ { if (in_section) exit }
    in_section && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"
tag_version="${tag_ref#refs/tags/v}"

echo "Cargo.toml version: $package_version"
echo "Git tag version: $tag_version"

if [[ -z "$package_version" ]]; then
  echo "❌ Could not resolve workspace.package.version from Cargo.toml"
  exit 1
fi

if [[ "$package_version" != "$tag_version" ]]; then
  echo "❌ Tag v$tag_version does not match Cargo.toml workspace version $package_version"
  exit 1
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'version=%s\n' "$package_version" >> "$GITHUB_OUTPUT"
else
  printf 'version=%s\n' "$package_version"
fi
