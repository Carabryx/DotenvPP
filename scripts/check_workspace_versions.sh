#!/usr/bin/env bash
# Verify that workspace path-dependency versions in [workspace.dependencies]
# match [workspace.package] version. Catches the "forgot to bump" mistake
# before cargo publish fails.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cargo_toml="$script_dir/../Cargo.toml"

# Extract workspace.package.version
pkg_version="$(
  awk '
    /^\[workspace\.package\]$/ { in_section = 1; next }
    /^\[/ { if (in_section) exit }
    in_section && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$cargo_toml"
)"

if [[ -z "$pkg_version" ]]; then
  echo "❌ Could not read workspace.package.version from Cargo.toml"
  exit 1
fi

echo "workspace.package.version = $pkg_version"

# Check each path dependency in [workspace.dependencies] has a matching version
errors=0
while IFS= read -r line; do
  # Extract crate name and version from lines like:
  #   dotenvpp-parser = { path = "...", version = "=0.0.2" }
  crate_name="$(echo "$line" | awk -F'=' '{print $1}' | tr -d ' ')"
  dep_version="$(echo "$line" | grep -oP 'version\s*=\s*"\K[^"]+' || true)"

  if [[ -z "$dep_version" ]]; then
    echo "❌ $crate_name: path dependency missing version field"
    errors=$((errors + 1))
    continue
  fi

  # Strip leading = for comparison (=0.0.2 → 0.0.2)
  clean_version="${dep_version#=}"

  if [[ "$clean_version" != "$pkg_version" ]]; then
    echo "❌ $crate_name: version \"$dep_version\" does not match workspace version \"$pkg_version\""
    errors=$((errors + 1))
  else
    echo "✅ $crate_name: $dep_version"
  fi
done < <(awk '/^\[workspace\.dependencies\]/{p=1;next} /^\[/{p=0} p' "$cargo_toml" | grep -E 'path\s*=' | grep -v '^\s*#')

if [[ "$errors" -gt 0 ]]; then
  echo ""
  echo "❌ $errors workspace dependency version(s) out of sync."
  echo "   Update [workspace.dependencies] in Cargo.toml to match workspace.package.version = \"$pkg_version\""
  exit 1
fi

echo ""
echo "✅ All workspace dependency versions match $pkg_version"
