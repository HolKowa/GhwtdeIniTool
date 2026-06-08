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
- `GhwtdeIniTool/src/components/ScanToast.tsx` renders compact scan completion
  and error toasts.
- `GhwtdeIniTool/src/components/ScanWizard.tsx` renders the scan confirmation
  wizard and category move preview list.
- `GhwtdeIniTool/src/components/SettingsDialog.tsx` renders project settings.
- `GhwtdeIniTool/src/components/UpdateDialog.tsx` renders updater states.
- `GhwtdeIniTool/src/hooks/useProjectSettings.ts` owns settings state and
  persistence calls.
- `GhwtdeIniTool/src/hooks/useModsScanner.ts` owns MODS scan action state.
- `GhwtdeIniTool/src/hooks/useAppUpdater.ts` owns updater state and actions.
- `GhwtdeIniTool/src/services/projectSettingsApi.ts` wraps the Tauri settings
  commands.
- `GhwtdeIniTool/src/services/scanModsApi.ts` wraps the Tauri MODS scan preview
  and execute commands.
- `GhwtdeIniTool/src/services/modsFolderDialog.ts` wraps Tauri folder pickers.
- `GhwtdeIniTool/src/types/projectSettings.ts` defines the frontend settings
  shape returned by Rust.
- `GhwtdeIniTool/src/types/scanMods.ts` defines the frontend MODS scan preview
  and result shapes.
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
- The Scan MODS folder button opens a wizard that first previews category move
  files relative to the configured MODS folder, then moves files only after the
  user confirms. The wizard scales with the app window while keeping preview
  content scrollable. Completion and errors are shown as compact toasts.
- The settings dialog lets the user choose a MODS folder, choose or clear the
  extra folder used for moved categories, and edit the keep-pattern string. The
  `Keep all` button saves `*`, and `Keep default` restores the default pattern.
- When no available MODS folder is selected, the extra-folder and keep-pattern
  settings are disabled until the user chooses a valid MODS folder.
- When a configured extra categories folder is available, the scan wizard
  previews and then moves each discovered `category.ini` and sibling
  `*.img.xen` files into a new folder under the extra folder named after the
  source category folder. Destination folder conflicts are auto-renamed with a
  numeric suffix.
- When no extra categories folder is available, the MODS scan reports discovered
  categories but does not move files.
- Settings changes are saved immediately through `save_project_settings`; there
  is no separate Apply or Save button.
- The frontend default pattern is
  `song.ini,*_song.pak.xen,*.fsb.xen,category.ini,*.img.xen`, matching the Rust
  default.
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
  `preview_scan_mods_folder`, and `scan_mods_folder` Tauri commands.
- Stores project settings in `ghwtdeinitool.ini` next to the executable under a
  `[project]` section.
- Reads and writes `mods_dir`, `categories_extra_dir`, and
  `keep_only_files_pattern`.
- Reports `mods_dir_available` and `categories_extra_dir_available` by checking
  whether the stored paths still exist as directories.
- Requires `mods_dir` to be an existing directory before saving settings.
- Allows `categories_extra_dir` to be empty, and creates it when set to a path
  that does not exist.
- Rejects a `categories_extra_dir` that is the MODS folder or inside the MODS
  folder.
- Recursively scans the MODS folder for `category.ini`; preview returns
  MODS-relative source file paths for category move files, and confirmed moves
  include the INI file and case-insensitive sibling `*.img.xen` files only,
  leaving source folders in place.
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
- Rust scan helper tests cover recursive discovery, category move previews,
  category-data moves, destination auto-renaming, source-folder preservation,
  and scan-only behavior.

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
