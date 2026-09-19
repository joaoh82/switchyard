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

| System                                   | Artifacts                                              | Signing                                                                                |
| ---------------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| Linux (x86_64)                           | `.AppImage`, `.deb`, `.rpm`; `yardsort-bin` on the AUR | —                                                                                      |
| macOS (universal: Apple Silicon + Intel) | `.dmg`, `.app.tar.gz`                                  | Signed and notarized when the Apple secrets below are set; otherwise an unsigned build |
| Windows (x86_64)                         | `-setup.exe` (NSIS), `.msi`                            | Not signed yet — users see a SmartScreen warning                                       |

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

## Arch Linux: the AUR package

Each full release (not pre-releases) is also published to the
[Arch User Repository](https://aur.archlinux.org/packages/yardsort-bin) as **`yardsort-bin`**, which
repackages the release's `.deb`. The last job of the workflow renders `packaging/aur/PKGBUILD.in`
with the version and checksums, **builds the package in an Arch container to prove it works**, and
pushes `PKGBUILD` and `.SRCINFO` to the AUR.

It needs one secret, `AUR_SSH_PRIVATE_KEY`: the private half of an SSH key whose public half is
registered on the maintainer's AUR account (_My Account → SSH Public Key_). Use a key made only
for this:

```sh
ssh-keygen -t ed25519 -N "" -C "yardsort release workflow" -f aur_deploy_key
gh secret set AUR_SSH_PRIVATE_KEY < aur_deploy_key
cat aur_deploy_key.pub          # paste this into your AUR account, then delete both files
```

Without the secret the job says so and does nothing. To publish by hand instead:

```sh
git clone ssh://aur@aur.archlinux.org/yardsort-bin.git /tmp/yardsort-bin
scripts/aur-render.sh 0.2.0 /tmp/yardsort-bin      # writes PKGBUILD and .SRCINFO
cd /tmp/yardsort-bin && makepkg -f && git add PKGBUILD .SRCINFO && git commit -m "Update to 0.2.0" && git push
```

The name is `yardsort-bin` because it ships a prebuilt binary; a build-from-source `yardsort`
package is welcome from anyone who wants to maintain one.

## Windows signing

Deliberately not set up: certificates cost money and the project has none. If that changes,
Tauri's [Windows signing guide](https://tauri.app/distribute/sign/windows/) applies and the
workflow needs the certificate passed to the build step.

## Updates: signing and `latest.json`

Installed copies of Yardsort find new versions by reading
`https://github.com/joaoh82/yardsort/releases/latest/download/latest.json`, and install one only
if its signature verifies against the public key in `src-tauri/tauri.conf.json`
(`plugins.updater.pubkey`).

The release workflow builds with `src-tauri/tauri.release.conf.json`, which turns on
`createUpdaterArtifacts`: the self-updating bundles (AppImage, the macOS `.app.tar.gz`, the NSIS
and MSI installers) each get a `.sig`, and `latest.json` lists them. The publish job **refuses to
make a release public if `latest.json` is missing a platform** — such a release would silently
strand everyone on older versions.

It needs two secrets: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

> **This key is the project's most important secret.** If it is **lost**, no installed copy can
> ever auto-update again — users must reinstall by hand. If it **leaks**, anyone who can also
> publish a release here can ship code to every user. Keep an offline backup. To rotate it, ship a
> release signed with the _old_ key whose app carries the _new_ public key, then switch the secrets.

Local builds (`just build`) do not create updater artifacts and need no key. Which copies update
themselves is decided by how they were packaged (`src-tauri/src/updates.rs`): `.deb`, `.rpm` and
the AUR package are owned by a package manager and are only told that a new version exists.

To rehearse the whole flow against your own server, start a release build with
`YARDSORT_UPDATE_ENDPOINT=https://…/latest.json`.

## Not there yet

- **More package managers** — Homebrew cask, winget, Flatpak.
