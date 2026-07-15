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
- `jq` if you want to generate local Linux updater fixtures.

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
GHWTDE_UPDATER_REPOSITORY=HolKowa/GhwtdeIniLoader

# Optional direct updater endpoint override.
# Useful for local updater testing.
GHWTDE_UPDATER_ENDPOINT=http://localhost:1420/updater-local/latest.json

# Optional app version override.
# Set this lower than latest.json's version to force an update to appear.
GHWTDE_APP_VERSION=0.0.0

# Required for signed updater artifacts.
TAURI_SIGNING_PRIVATE_KEY="your private key here"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your private key password here"
```

The `pnpm tauri` script loads `.env` and `.env.local`. If
`GHWTDE_UPDATER_ENDPOINT`, `GHWTDE_UPDATER_REPOSITORY`, or `GITHUB_REPOSITORY`
is set, it writes `src-tauri/tauri.updater.generated.json` and passes it to
Tauri for commands such as `dev` and `build`.

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

The app checks for updates on launch.

- If no update is available, nothing is shown.
- If the update server cannot be reached during the initial check, nothing is shown.
- If an update is available, a popup is shown with `Update` and `Skip`.
- `Skip` closes the popup and shows no extra message.
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

Local updater testing currently targets Linux `.deb` artifacts. Create signing
keys first, then use those signing values while building the local fixture.

1. Put valid signing values in `.env.local`:

```sh
TAURI_SIGNING_PRIVATE_KEY="your private key here"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your private key password here"
```

2. Build a signed local updater fixture:

```sh
pnpm updater:build-linux 99.0.0
```

This creates:

```text
public/updater-local/ghwtdeinitool_99.0.0_amd64.deb
public/updater-local/latest.json
```

3. Set local updater settings in `.env.local`:

```sh
GHWTDE_UPDATER_ENDPOINT=http://localhost:1420/updater-local/latest.json
GHWTDE_APP_VERSION=0.0.0
```

4. Start the app:

```sh
pnpm tauri dev
```

Because `GHWTDE_APP_VERSION` is lower than `99.0.0`, the updater should show the
update popup.

## Releases

The release workflow is defined in `../.github/workflows/build.yml`.

On pushes to `integration`:

- Windows is built.
- Build artifacts are uploaded to the workflow run.
- No GitHub Release is created.

On pushes to `main`:

- Windows is built.
- A GitHub Release is created automatically by `tauri-apps/tauri-action`.
- The release tag/name uses the app version from Tauri.
- `latest.json` is included for the updater.

Manual workflow dispatch can also build Linux artifacts when `build-linux` is
enabled.

Before creating a release:

1. Update the app version in `src-tauri/tauri.conf.json`.
2. Make sure no local-only updater overrides are being used for the release:

```sh
GHWTDE_UPDATER_ENDPOINT
GHWTDE_APP_VERSION
```

3. Make sure the GitHub repository has these secrets:

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

4. Regenerate the third-party notice:

```sh
cargo install --locked --features cli cargo-about # once
pnpm licenses:generate
```

5. Build locally if possible:

```sh
pnpm build
pnpm tauri build
```

6. Push or merge to `main`.

## Useful Commands

```sh
pnpm install
pnpm tauri dev
pnpm build
pnpm tauri build
pnpm updater:build-linux 99.0.0
cd src-tauri && cargo check
```

## Generated And Ignored Files

These are intentionally not committed:

- `.env`, `.env.local`, and other `.env.*` files except `.env.example`
- `node_modules/`
- `dist/`
- `public/updater-local/`
- `src-tauri/tauri.updater.generated.json`
- Tauri build output under `src-tauri/target/`
