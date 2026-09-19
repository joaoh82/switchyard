#!/usr/bin/env bash
# Render the AUR package (PKGBUILD, and .SRCINFO where makepkg exists) for a published release.
#
# usage: scripts/aur-render.sh <version> <output-dir>      e.g. 0.2.0 /tmp/yardsort-bin
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/aur-render.sh <version> <output-dir>}"
out="${2:?usage: scripts/aur-render.sh <version> <output-dir>}"
mkdir -p "$out"

# pkgver may not contain a hyphen: 0.3.0-beta.1 becomes 0.3.0_beta.1
pkgver="${version//-/_}"
base="https://github.com/joaoh82/yardsort"
sum() { curl --fail --silent --show-error --location "$1" | sha256sum | cut -d' ' -f1; }

deb_sum="$(sum "$base/releases/download/v$version/Yardsort_${version}_amd64.deb")"
license_sum="$(sum "https://raw.githubusercontent.com/joaoh82/yardsort/v$version/LICENSE")"

sed -e "s/@PKGVER@/$pkgver/g" -e "s/@VERSION@/$version/g" \
    -e "s/@SHA256_DEB@/$deb_sum/" -e "s/@SHA256_LICENSE@/$license_sum/" \
    packaging/aur/PKGBUILD.in > "$out/PKGBUILD"

if command -v makepkg >/dev/null; then
  (cd "$out" && makepkg --printsrcinfo > .SRCINFO)
fi
echo "Rendered yardsort-bin $pkgver into $out"
