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
   - flips the release to public only once **all four** platforms succeeded.

   A failed platform therefore leaves a draft you can inspect or delete, never a
   half-populated public release.

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

The upstream workflow, kept unchanged. Use it when you have the SignPath
credentials, since it is the only path that produces signed Windows binaries.

It is `workflow_dispatch` only and must be triggered from `dev`. It takes its
version from `espanso/Cargo.toml` rather than from a tag, requires the tag to
already exist (`--verify-tag`), publishes as a prerelease immediately, and does
not upload the macOS artifact. Signing needs `secrets.SIGNPATH_API_TOKEN` and a
SignPath organization — without them the Windows job fails.
