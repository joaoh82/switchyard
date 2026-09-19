#!/usr/bin/env bash
# Render the Homebrew cask for a published release.
#
# usage: scripts/homebrew-render.sh <version> <output-file>     e.g. 0.3.1 /tmp/tap/Casks/yardsort.rb
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/homebrew-render.sh <version> <output-file>}"
out="${2:?usage: scripts/homebrew-render.sh <version> <output-file>}"
url="https://github.com/joaoh82/yardsort/releases/download/v$version/Yardsort_${version}_universal.dmg"
# macOS has `shasum`, Linux has `sha256sum`.
if command -v sha256sum >/dev/null; then hash=(sha256sum); else hash=(shasum -a 256); fi
sum="$(curl --fail --silent --show-error --location "$url" | "${hash[@]}" | cut -d' ' -f1)"

mkdir -p "$(dirname "$out")"
sed -e "s/@VERSION@/$version/g" -e "s/@SHA256@/$sum/" packaging/homebrew/yardsort.rb.in > "$out"
echo "Rendered cask yardsort $version ($sum) into $out"
