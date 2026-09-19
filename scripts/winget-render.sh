#!/usr/bin/env bash
# Render the three winget manifests for a published release.
#
# usage: scripts/winget-render.sh <version> <output-dir>
#   e.g. scripts/winget-render.sh 0.3.1 /tmp/winget   ->   /tmp/winget/manifests/j/joaoh82/Yardsort/0.3.1/
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/winget-render.sh <version> <output-dir>}"
out="${2:?usage: scripts/winget-render.sh <version> <output-dir>}/manifests/j/joaoh82/Yardsort/$version"
url="https://github.com/joaoh82/yardsort/releases/download/v$version/Yardsort_${version}_x64-setup.exe"

if command -v sha256sum >/dev/null; then hash=(sha256sum); else hash=(shasum -a 256); fi
sum="$(curl --fail --silent --show-error --location "$url" | "${hash[@]}" | cut -d' ' -f1 | tr '[:lower:]' '[:upper:]')"
date="$(curl --fail --silent --show-error "https://api.github.com/repos/joaoh82/yardsort/releases/tags/v$version" \
  | sed -n 's/.*"published_at": *"\([0-9-]*\)T.*/\1/p' | head -1)"
[ -n "$date" ] || date="$(date -u +%F)"

mkdir -p "$out"
for template in packaging/winget/*.in; do
  sed -e "s/@VERSION@/$version/g" -e "s/@SHA256@/$sum/" -e "s/@DATE@/$date/" "$template" \
    > "$out/$(basename "${template%.in}")"
done
echo "Rendered winget manifests for joaoh82.Yardsort $version into $out"
