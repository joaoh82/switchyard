#!/usr/bin/env bash
# Open a pull request against microsoft/winget-pkgs for a published release, using the GitHub API
# only (the repository is far too large to clone for three small files).
#
# usage: GH_TOKEN=<token with public_repo> scripts/winget-submit.sh <version>
# The token's account must own a fork of microsoft/winget-pkgs.
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/winget-submit.sh <version>}"
upstream="microsoft/winget-pkgs"
id="joaoh82.Yardsort"
path="manifests/j/joaoh82/Yardsort"
user="$(gh api user -q .login)"
fork="$user/winget-pkgs"
branch="$id-$version"

if gh api "repos/$upstream/contents/$path/$version" >/dev/null 2>&1; then
  echo "winget already has $id $version."; exit 0
fi
if [ "$(gh pr list -R "$upstream" --author "$user" --state open --search "$id in:title" --json number -q length)" != "0" ]; then
  echo "::warning::A pull request for $id is still open at $upstream; not opening another. Submit $version once it is merged (Actions -> Package managers)."
  exit 0
fi
# "New package" until the first version has been accepted, "New version" after.
if gh api "repos/$upstream/contents/$path" >/dev/null 2>&1; then kind="New version"; else kind="New package"; fi

work="$(mktemp -d)"
scripts/winget-render.sh "$version" "$work"

gh api "repos/$fork/merge-upstream" -X POST -f branch=master >/dev/null
base="$(gh api "repos/$upstream/git/ref/heads/master" -q .object.sha)"
gh api "repos/$fork/git/refs" -X POST -f ref="refs/heads/$branch" -f sha="$base" >/dev/null
for file in "$work/$path/$version"/*.yaml; do
  gh api "repos/$fork/contents/$path/$version/$(basename "$file")" -X PUT \
    -f message="$kind: $id version $version" -f branch="$branch" \
    -f content="$(base64 < "$file" | tr -d '\n')" >/dev/null
done

gh pr create -R "$upstream" --head "$user:$branch" --base master \
  --title "$kind: $id version $version" \
  --body "$kind: **$id** version **$version**, submitted by the project's release workflow.

Release: https://github.com/joaoh82/yardsort/releases/tag/v$version

- [x] This PR only modifies one (1) manifest
- [x] Manifest conforms to the 1.12 schema
- [ ] Validated / tested locally with winget — not done: generated on Linux; relying on the validation pipeline"
