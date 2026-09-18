# Releasing

A release is a **version tag**. Pushing `v0.2.0` makes GitHub Actions build installers for Linux,
macOS and Windows and publish them as a GitHub release — no further clicks.

While the builds run, the release exists only as a draft. It is made public by the last job, and
only if **every** platform built; if one fails, the draft stays hidden for you to inspect, fix and
re-run.

## Cutting a release

```sh
just release 0.2.0
```

That recipe checks the tree is clean and on `main`, runs `just check`, sets the version in
`Cargo.toml`, `package.json` and `src-tauri/tauri.conf.json`, commits `Release v0.2.0`, tags it and
pushes both. Then:

1. Watch the run: `just ci-watch`, or the repository's _Actions_ tab. About 20 minutes.
2. The release appears under _Releases_ with notes generated from the commits since the last tag.
   Edit them afterwards if you like.

If a build fails: fix the cause, then re-run the failed jobs from the _Actions_ tab (or run the
_Release_ workflow by hand with the same tag). To abandon the attempt instead, delete the draft
and the tag: `gh release delete v0.2.0 --cleanup-tag`.

Versions follow [semver](https://semver.org). A tag with a suffix — `v0.2.0-beta.1` — is marked as
a pre-release automatically.

To rebuild without a new tag, run the _Release_ workflow by hand (_Actions → Release → Run
workflow_) and give it an existing tag.

## What gets built

| System                                   | Artifacts                   | Signing                                                                                |
| ---------------------------------------- | --------------------------- | -------------------------------------------------------------------------------------- |
| Linux (x86_64)                           | `.AppImage`, `.deb`, `.rpm` | —                                                                                      |
| macOS (universal: Apple Silicon + Intel) | `.dmg`, `.app.tar.gz`       | Signed and notarized when the Apple secrets below are set; otherwise an unsigned build |
| Windows (x86_64)                         | `-setup.exe` (NSIS), `.msi` | Not signed yet — users see a SmartScreen warning                                       |

## macOS signing and notarization

Needs a paid Apple Developer account. Set these repository secrets
(_Settings → Secrets and variables → Actions_); the workflow picks them up with no other change:

| Secret                       | What it is                                                                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Your **Developer ID Application** certificate, exported from Keychain Access as a `.p12`, then base64-encoded: `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | The password you gave the `.p12` export                                                                                                       |
| `APPLE_SIGNING_IDENTITY`     | e.g. `Developer ID Application: Your Name (TEAMID)` — `security find-identity -v -p codesigning` lists it                                     |
| `APPLE_ID`                   | The Apple ID email of the developer account                                                                                                   |
| `APPLE_PASSWORD`             | An **app-specific password** for that Apple ID, from [account.apple.com](https://account.apple.com) → _Sign-In and Security_                  |
| `APPLE_TEAM_ID`              | The ten-character team id, from the developer portal's _Membership_ page                                                                      |

Creating the certificate: in Xcode → _Settings → Accounts → Manage Certificates → + → Developer ID
Application_; or in the developer portal under _Certificates_. Then export it from Keychain Access
(select the certificate **with its private key** → _Export_).

With `gh`:

```sh
gh secret set APPLE_CERTIFICATE < <(base64 -i cert.p12)
gh secret set APPLE_CERTIFICATE_PASSWORD
gh secret set APPLE_SIGNING_IDENTITY --body "Developer ID Application: Your Name (TEAMID)"
gh secret set APPLE_ID --body "you@example.com"
gh secret set APPLE_PASSWORD
gh secret set APPLE_TEAM_ID --body "ABCDE12345"
```

Until they are set, macOS builds still succeed but are unsigned, and Gatekeeper will refuse them
unless the user removes the quarantine attribute (see
[troubleshooting](guide/troubleshooting.md#macos)).

## Windows signing

Deliberately not set up: certificates cost money and the project has none. If that changes,
Tauri's [Windows signing guide](https://tauri.app/distribute/sign/windows/) applies and the
workflow needs the certificate passed to the build step.

## Not there yet

- **Auto-update.** Needs a signing key for update manifests, kept safe forever (losing it strands
  every installed copy). Tracked on the [roadmap](design/05-roadmap.md).
- **Package managers** — AUR, Homebrew cask, winget, Flatpak.
