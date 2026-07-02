# Repository Structure

This repository currently contains a basic Tauri 2 desktop application in the
`GhwtdeIniTool/` directory, plus repository-level CI configuration.

## Top Level

```text
.
|-- .github/
|   `-- workflows/
|       `-- build.yml
|-- GhwtdeIniTool/
|   |-- src/
|   |-- src-tauri/
|   |-- public/
|   |-- package.json
|   |-- pnpm-lock.yaml
|   |-- pnpm-workspace.yaml
|   |-- vite.config.ts
|   |-- tsconfig.json
|   |-- tsconfig.node.json
|   |-- index.html
|   `-- README.md
`-- LICENSE
```

Generated or dependency directories such as `node_modules/`, `dist/`, and
`src-tauri/target/` may also exist locally, but they are build artifacts rather
than source structure.

## Frontend

The frontend is a React 19 and TypeScript app built with Vite.

Important files:

- `GhwtdeIniTool/src/main.tsx` mounts the React application.
- `GhwtdeIniTool/src/App.tsx` coordinates startup, settings, and update UI.
- `GhwtdeIniTool/src/App.css` contains the current app styling.
- `GhwtdeIniTool/src/components/CleanModsWizard.tsx` renders the two-step MODS
  cleanup wizard for keep-pattern entry and delete confirmation.
- `GhwtdeIniTool/src/components/ScanToast.tsx` renders compact scan completion
  and error toasts.
- `GhwtdeIniTool/src/components/ScanWizard.tsx` renders the `song.ini` scan and
  repair wizard.
- `GhwtdeIniTool/src/components/SettingsDialog.tsx` renders project settings.
- `GhwtdeIniTool/src/components/UpdateDialog.tsx` renders updater states.
- `GhwtdeIniTool/src/hooks/useProjectSettings.ts` owns settings state and
  persistence calls.
- `GhwtdeIniTool/src/hooks/useModsCleaner.ts` owns keep-pattern cleanup state.
- `GhwtdeIniTool/src/hooks/useModsScanner.ts` owns `song.ini` scan action state.
- `GhwtdeIniTool/src/hooks/useAppUpdater.ts` owns updater state and actions.
- `GhwtdeIniTool/src/services/projectSettingsApi.ts` wraps the Tauri settings
  commands.
- `GhwtdeIniTool/src/services/scanModsApi.ts` wraps the Tauri keep-pattern
  delete and `song.ini` scan/validation commands.
- `GhwtdeIniTool/src/services/modsFolderDialog.ts` wraps Tauri folder pickers.
- `GhwtdeIniTool/src/types/projectSettings.ts` defines the frontend settings
  shape returned by Rust.
- `GhwtdeIniTool/src/types/scanMods.ts` defines the frontend MODS scan,
  keep-pattern delete, and `song.ini` scan/validation result shapes.
- `GhwtdeIniTool/src/utils/keepOnlyFilesPattern.ts` defines the default
  keep-pattern and frontend validation helper.
- `GhwtdeIniTool/vite.config.ts` configures Vite for Tauri development on port
  `1420`.

Current frontend behavior:

- On React startup, `App.tsx` calls `checkForUpdates()` and `loadSettings()`.
- Update checks use `@tauri-apps/plugin-updater`. If an update is available,
  the update dialog can start the download, show percentage progress, install
  the downloaded update, relaunch through `@tauri-apps/plugin-process`, skip the
  update, or show an error.
- If the updater check fails, the frontend currently treats it as no update and
  does not show an error dialog.
- Settings are loaded from the Rust `load_project_settings` command.
- If no MODS folder is configured, or the configured MODS folder is unavailable,
  settings status becomes `needs-folder` and the settings dialog opens
  automatically. The dialog cannot be closed until `mods_dir_available` is true.
- The settings button can reopen the dialog after startup.
- The Clean MODS folder button opens a two-step wizard that asks for a
  keep-pattern, previews MODS files that do not match it, and deletes only after
  user confirmation. The pattern defaults to
  `song*.ini,*_song.pak.xen,*.fsb.xen,category*.ini,*.img.xen,Readme.txt`
  each time and is not saved.
- The Scan MODS folder button opens a wizard that parses `song.ini` files,
  stores valid parsed results in backend memory, and lets the user repair faulty
  files before finishing. The wizard scales with the app window while keeping
  preview content scrollable. Completion and errors are shown as compact toasts.
- The settings dialog lets the user choose a MODS folder.
- Settings changes are saved immediately through `save_project_settings`; there
  is no separate Apply or Save button.
- Keep-pattern entries are comma-separated, trimmed, matched case-insensitively
  against file names, and support `*` wildcards. An empty keep-pattern keeps
  all files.
- The clean wizard validates the keep-pattern input before previewing and
  rejects path separators or reserved filename characters.
- The frontend displays unavailable configured folders as muted paths but keeps
  the stored path visible.

Frontend scripts from `GhwtdeIniTool/package.json`:

- `pnpm dev` starts the Vite development server.
- `pnpm build` runs TypeScript checking and builds the frontend.
- `pnpm preview` previews the built frontend.
- `pnpm tauri` runs the Tauri CLI.

## Backend

The backend is the Rust side of the Tauri app in `GhwtdeIniTool/src-tauri/`.

Important files:

- `GhwtdeIniTool/src-tauri/src/main.rs` starts the Tauri binary and calls the
  library entry point.
- `GhwtdeIniTool/src-tauri/src/lib.rs` configures the Tauri builder, plugins,
  and command handlers.
- `GhwtdeIniTool/src-tauri/Cargo.toml` defines Rust dependencies and crate
  metadata.
- `GhwtdeIniTool/src-tauri/tauri.conf.json` defines app metadata, windows,
  bundling, updater settings, and frontend build integration.
- `GhwtdeIniTool/src-tauri/capabilities/default.json` defines default Tauri
  permissions.

Current backend behavior:

- Registers the Tauri opener, updater, process, and dialog plugins.
- Exposes `load_project_settings`, `save_project_settings`,
  `preview_keep_only_files_delete`, `delete_keep_only_files`,
  `scan_song_ini_files`, and `validate_song_ini_file` Tauri commands.
- Stores project settings in `ghwtdeinitool.ini` next to the executable under a
  `[project]` section.
- Reads and writes `mods_dir`. Legacy `keep_only_files_pattern` entries in old
  settings files are ignored.
- Reports `mods_dir_available` by checking whether the stored path still exists
  as a directory.
- Requires `mods_dir` to be an existing directory before saving settings.
- Recursively previews files that do not match the keep-pattern argument passed
  by the clean wizard; confirmed deletes validate each MODS-relative path
  before removing only those files.
- Recursively scans case-insensitive `song.ini` files, stores
  valid parsed INI data in non-persistent backend memory, returns faulty file
  contents and parse errors to the wizard, rejects duplicate keys within the
  same section, requires exact `[ModInfo]` and `[SongInfo]` sections plus a
  non-empty `Checksum` entry in `[SongInfo]`, auto-corrects canonical casing for
  known `ModInfo`/`SongInfo` keys while allowing extra unknown keys, and
  validates repaired contents before writing them back to disk.
- Uses `tauri.conf.json` to configure bundling and updater artifacts.

## Build And Release

The repository has one GitHub Actions workflow:

- `.github/workflows/build.yml`

Current workflow behavior:

- Builds Windows artifacts on pushes to `main` and `integration`.
- Can optionally build Linux artifacts through manual workflow dispatch.
- Uses Node.js 24, pnpm, and Rust.
- Builds the Tauri app with signing secrets.
- Uploads installer and portable artifacts.
- Creates a GitHub release when pushing to `main`.

## Tests

No dedicated test setup is present at the moment.

What was checked:

- No `test` script exists in `GhwtdeIniTool/package.json`.
- No frontend test framework dependency such as Vitest, Jest, Playwright, or
  Testing Library is listed.
- No `*.test.*` or `*.spec.*` files were found.
- Rust scan helper tests cover keep-pattern delete previews, safe confirmed
  deletes, `song.ini` parsing, faulty file reporting, validation writes, store
  updates, duplicate key rejection, required `song.ini` fields, canonical key
  casing auto-correction, and unsafe repair path rejection.

Useful current validation commands:

```sh
cd GhwtdeIniTool
pnpm build
cd src-tauri
cargo test
cargo check
```

These are build and compile checks, not a replacement for unit or integration
tests.
