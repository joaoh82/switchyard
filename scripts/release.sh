#!/usr/bin/env bash
# Cut a release: bump the version everywhere, commit, tag, push. The tag triggers the Release
# workflow, which builds installers and publishes the release. See docs/releasing.md.
#
# usage: scripts/release.sh <version>      e.g. 0.2.0 or 0.2.0-beta.1
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:-}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "usage: scripts/release.sh <version>   (semver, without the leading v — e.g. 0.2.0)" >&2
  exit 2
fi
tag="v$version"

[ "$(git branch --show-current)" = "main" ] || { echo "Releases are cut from main." >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "The working tree is not clean." >&2; exit 1; }
git fetch --quiet --tags origin
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || { echo "main is not in sync with origin/main." >&2; exit 1; }
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then echo "Tag $tag already exists." >&2; exit 1; fi

just check
just bindings-check

# One version, three files.
# (perl rather than sed: GNU and BSD sed disagree about in-place edits and first-match ranges.)
perl -0pi -e 's/^version = "[^"]*"/version = "'"$version"'"/m' Cargo.toml
for file in package.json src-tauri/tauri.conf.json; do
  jq --arg v "$version" '.version = $v' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done
bunx prettier --write package.json src-tauri/tauri.conf.json >/dev/null
cargo update --workspace --quiet   # refresh the workspace's own entries in Cargo.lock

git add Cargo.toml Cargo.lock package.json src-tauri/tauri.conf.json
# The code may already say this version (the first release does): then the tag alone is the release.
if git diff --cached --quiet; then
  echo "Version is already $version; tagging the current commit."
else
  git commit -m "Release $tag"
fi
git tag -a "$tag" -m "Yardsort $tag"
git push origin main "$tag"

echo
echo "Pushed $tag. The Release workflow is building installers; it publishes the release by"
echo "itself once every platform has succeeded (about 20 minutes):"
echo "  https://github.com/joaoh82/yardsort/actions/workflows/release.yml"
echo "  https://github.com/joaoh82/yardsort/releases"
