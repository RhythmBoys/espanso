# Creating a release

Pushing a `v*` tag builds every platform and publishes the artifacts to a GitHub
Release. There are two workflows; pick the one that matches what you need.

## Tag-triggered release (`tag-release.yml`)

This is the automated path. Nothing to click.

### Step by step

1) Bump the version in `espanso/Cargo.toml` and merge it into `dev`.

   > **Do this first.** The Windows installer reads its version from
   > `espanso/Cargo.toml` (see `scripts/build_windows_installer.ps1`), not from
   > the tag. If the two disagree, the release is titled after the tag while the
   > installed app reports the manifest version. The workflow emits a warning on
   > mismatch but does not stop.

2) Tag the commit and push the tag:

   ```bash
   git tag v2.2.2
   git push origin v2.2.2
   ```

3) That's it. The workflow then:

   - creates the release as a **draft**,
   - builds and uploads Windows, Linux (X11 + deb) and macOS artifacts in
     parallel,
   - flips the release to public once **Windows and macOS** have succeeded.

   Only Windows and macOS gate publication. The `publish` job still waits for
   the Linux jobs to finish, so their artifacts are attached whenever they do
   succeed — but a Linux failure no longer holds the release back. The run is
   still marked failed and a warning names the missing platforms; re-running
   those jobs attaches their artifacts to the existing release.

   If Windows or macOS fails, the release stays a **draft**. Drafts are only
   visible to people with write access on the repository — if the Releases page
   looks empty after a tag push, check for a draft there before assuming
   nothing was created.

4) Share the news — make an announcement in the `espanso` discord.

### Artifacts

| Platform | Files |
| --- | --- |
| Windows | `Espanso-Win-Installer-x86_64.exe`, `Espanso-Win-Portable-x86_64.zip` |
| Linux X11 | `Espanso-X11.AppImage` |
| Linux deb | `espanso-debian-x11-amd64.deb`, `espanso-debian-wayland-amd64.deb` |
| macOS | `Espanso-Mac-Universal.zip` |

### These artifacts are not code signed

`tag-release.yml` has no SignPath step, so:

- **Windows**: SmartScreen warns on first run.
- **macOS**: the app is neither signed nor notarized. Gatekeeper blocks it when
  launched from `/Applications`; users must open it once via right-click → Open,
  or run
  `xattr -dr com.apple.quarantine /Applications/Espanso.app`.
  This is the reason the upstream workflow leaves the macOS upload commented
  out — if you need a distributable macOS build, sign it yourself.

The generated release notes state all of this.

## Signed release (`create-release-draft.yml`)

The upstream workflow. Use it when you have signing credentials.

It is `workflow_dispatch` only and must be triggered from `dev`. It takes its
version from `espanso/Cargo.toml` rather than from a tag, requires the tag to
already exist (`--verify-tag`), and publishes as a prerelease.

### Step by step

1) Run the `create-release-draft.yml` workflow. The CI builds, codesigns and
notarizes the macOS DMG automatically, using Auca's Apple Developer ID
Application certificate (individual enrollment — espanso doesn't have a
registered legal entity, so an org-owned Apple Developer account isn't an
option) stored in the repo's GitHub Actions secrets (`MACOS_CERTIFICATE`,
`MACOS_CERTIFICATE_PWD`, `MACOS_CERTIFICATE_NAME`, `MACOS_CI_KEYCHAIN_PWD`,
`APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`). As with
Federico's certificate before it, this ties signing to one maintainer's
Apple ID; if Auca's certificate ever expires, is revoked, or he steps away,
someone will need to re-enroll and refresh these secrets. If the secrets are
ever missing or stale, the `macos` job step "Codesign app bundle" will fail
and the DMG will need to be signed manually as a fallback.

Windows signing needs `secrets.SIGNPATH_API_TOKEN` and a SignPath organization
— without them the Windows job fails.

2) Wait until the workflow finishes...

3) Update the description and hit publish!

4) Share the news — make an announcement in the `espanso` discord.
