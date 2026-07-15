# Development Guide

GhwtdeIniTool is a desktop app built with React, TypeScript, Vite, and Tauri 2.
The frontend lives in `GhwtdeIniTool/src/` and the Tauri/Rust backend lives in
`GhwtdeIniTool/src-tauri/`.

Unless stated otherwise, run app commands from the app directory:

```sh
cd GhwtdeIniTool
```

## Requirements

- Node.js 24, matching the GitHub Actions workflow.
- pnpm 11.x.
- Rust stable.
- `cargo-about` to regenerate third-party notices for releases.
- Tauri system dependencies for your OS.
- `jq` if you want to generate local Linux AppImage updater fixtures.

Install frontend dependencies from the app directory:

```sh
pnpm install
```

## Environment

Create a local env file from the example:

```sh
cp .env.example .env.local
```

`.env.local` is ignored by git. Keep signing keys and local-only overrides there.

Useful variables:

```sh
# Repository used to generate the GitHub Releases updater endpoint.
GHWTDE_UPDATER_REPOSITORY=HolKowa/GhwtdeIniTool

# Optional direct updater endpoint override.
# Useful for local updater testing.
GHWTDE_UPDATER_ENDPOINT=http://localhost:1420/updater-local/latest.json

# Development-only app version override for updater testing.
# It applies only to `pnpm tauri dev`, never to packaged builds or GitHub Actions.
# Set this lower than latest.json's version to force an update to appear.
GHWTDE_APP_VERSION=0.0.0

# Required for signed updater artifacts.
TAURI_SIGNING_PRIVATE_KEY="your private key here"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your private key password here"
```

The `pnpm tauri` script loads `.env` and `.env.local`. If
`GHWTDE_UPDATER_ENDPOINT`, `GHWTDE_UPDATER_REPOSITORY`, or `GITHUB_REPOSITORY`
is set, it writes `src-tauri/tauri.updater.generated.json` and passes it to
Tauri for commands such as `dev` and `build`. `GHWTDE_APP_VERSION` is included
in that generated config only for `dev`.

## Versioning

`src-tauri/tauri.conf.json` is the single release-version source. Update its
top-level `version` field for every release. That version is used by local
packaged builds and GitHub Actions, including the About dialog, bundle names,
GitHub Release tag/name, and updater metadata.

`GHWTDE_APP_VERSION` is a development-only override for `pnpm tauri dev`; do
not use it to set a release version. It cannot change a packaged build or a
GitHub Actions artifact.

## Development

Run the Tauri desktop app:

```sh
pnpm tauri dev
```

This starts Vite on port `1420` and opens the Tauri window.

Temporary dev scan exception: debug/Tauri dev runs currently suppress missing
`Content/MUSIC` warnings when the folder is absent, so local development can
omit bulky MUSIC assets. If `Content/MUSIC` exists, its files are still
validated. Remove this exception from code and docs when local development no
longer omits MUSIC assets.

## Project Settings INI

The app stores the last selected MODS folder in `ghwtdeinitool.ini` next to the
running executable.

In development, that file is usually here:

```text
src-tauri/target/debug/ghwtdeinitool.ini
```

In production, the file is next to the installed app executable. On Windows,
that is typically the same directory as `ghwtdeinitool.exe`. If the app is
installed in a protected location, writing this file may require a writable app
directory or elevated permissions.

Run only the frontend dev server:

```sh
pnpm dev
```

Build only the frontend:

```sh
pnpm build
```

Check the Rust/Tauri backend:

```sh
cd src-tauri
cargo check
```

## Updater Behavior

The app checks for updates on launch by default. Users can disable this from
the update popup or Settings; the preference is stored in `ghwtdeinitool.ini`.

- If no update is available, nothing is shown.
- If the update server cannot be reached during the initial check, nothing is shown.
- If an update is available, a popup is shown with `Update`, `Skip`, and
  `Don't check again`.
- `Skip` closes the popup and shows no extra message.
- `Don't check again` disables future launch-time update checks until the
  Settings checkbox is re-enabled.
- Download/install errors are shown only after the user chooses to update.

## Signing Keys

Generate updater signing keys with the direct Tauri CLI:

```sh
./node_modules/.bin/tauri signer generate
```

You can also write keys to a file:

```sh
./node_modules/.bin/tauri signer generate --write-keys updater.key
```

Store the private key and password as:

- local development: `.env.local`
- GitHub Actions: repository secrets named `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

The public key must match `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`.

## Local Updater Testing

Local updater testing currently targets Linux AppImage artifacts. Create signing
keys first, then use those signing values while building the local fixture.

1. Put valid signing values in `.env.local`:

```sh
TAURI_SIGNING_PRIVATE_KEY="your private key here"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your private key password here"
```

2. Build a signed local updater fixture:

```sh
pnpm updater:build-linux
```

This creates:

```text
public/updater-local/ghwtdeinitool_0.9.0_amd64.AppImage
public/updater-local/latest.json
```

The fixture version comes from `src-tauri/tauri.conf.json`; update that file
before building a fixture for a different version.

3. Set local updater settings in `.env.local`:

```sh
GHWTDE_UPDATER_ENDPOINT=http://localhost:1420/updater-local/latest.json
GHWTDE_APP_VERSION=0.0.0
```

4. Start the app:

```sh
pnpm tauri dev
```

Because `GHWTDE_APP_VERSION` is lower than `0.9.0`, the updater should show the
update popup. This override applies only to this development run.

## Releases

The release workflow is defined in `../.github/workflows/build.yml`.

On pushes to `integration`:

- Windows is built.
- Build artifacts are uploaded to the workflow run.
- No GitHub Release is created.

On pushes to `main`:

- Windows installers and a signed x86_64 Linux AppImage are built.
- A GitHub Release is created automatically by `tauri-apps/tauri-action`.
- The release tag/name uses the app version from Tauri.
- `latest.json` includes updater metadata for Windows and the Linux AppImage.

On `integration`, use **Actions** → **Build & Release** → **Run workflow** and
enable **Also build for Linux** to upload a signed AppImage as a workflow
artifact. This test build does not create a GitHub Release or publish updater
metadata.

Before creating a release:

1. Update the app version in `src-tauri/tauri.conf.json`.
2. Make sure no local updater endpoint override is being used for the release:

```sh
GHWTDE_UPDATER_ENDPOINT
```

3. Make sure the GitHub repository has these secrets:

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

4. Install `cargo-about` once. `pnpm tauri build` generates and embeds the
   third-party notice automatically; `pnpm tauri dev` embeds a short
   development placeholder instead:

```sh
cargo install --locked --features cli cargo-about # once
```

Set `GHWTDE_LICENSE_TARGET` to a Rust target triple when a local release build
should embed notices for only that platform. GitHub Actions sets this for each
release job; leaving it unset generates the conservative all-platform notice.

5. Build locally if possible:

```sh
pnpm build
pnpm tauri build
```

To verify the Linux release artifact locally on Ubuntu or another Debian-based
development environment, install the same Tauri dependencies used by CI:

```sh
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Then build only the AppImage:

```sh
pnpm tauri build -- --bundles appimage
```

The output is `src-tauri/target/release/bundle/appimage/` and is suitable for
testing on x86_64 Ubuntu and Arch Linux.

6. Push or merge to `main`; CI publishes the Windows installers and Linux
   AppImage automatically.

## Useful Commands

```sh
pnpm install
pnpm tauri dev
pnpm build
pnpm tauri build
pnpm tauri build -- --bundles appimage
pnpm updater:build-linux
cd src-tauri && cargo check
```

## Generated And Ignored Files

These are intentionally not committed:

- `.env`, `.env.local`, and other `.env.*` files except `.env.example`
- `node_modules/`
- `dist/`
- `public/updater-local/`
- `src-tauri/tauri.updater.generated.json`
- `src-tauri/resources/THIRD_PARTY_LICENSES.txt` (generated for release builds)
- Tauri build output under `src-tauri/target/`
