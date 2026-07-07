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
- `GhwtdeIniTool/src/components/InstrumentAnalyzeWizard.tsx` renders the
  separate instrument sidecar analysis wizard and progress state.
- `GhwtdeIniTool/src/components/ScanToast.tsx` renders compact scan completion
  and error toasts.
- `GhwtdeIniTool/src/components/ScanWizard.tsx` renders the `song.ini` scan,
  repair, duplicate checksum, disabled-file conflict, and content layout
  wizard.
- `GhwtdeIniTool/src/components/ScannedSongsTable.tsx` renders the compact
  sortable/filterable parsed-song results table shown after a completed scan.
- `GhwtdeIniTool/src/components/SettingsDialog.tsx` renders project settings.
- `GhwtdeIniTool/src/components/UpdateDialog.tsx` renders updater states.
- `GhwtdeIniTool/src/hooks/useProjectSettings.ts` owns settings state and
  persistence calls.
- `GhwtdeIniTool/src/hooks/useModsCleaner.ts` owns keep-pattern cleanup state.
- `GhwtdeIniTool/src/hooks/useModsScanner.ts` owns `song.ini` scan and
  instrument analysis action state.
- `GhwtdeIniTool/src/hooks/useAppUpdater.ts` owns updater state and actions.
- `GhwtdeIniTool/src/services/projectSettingsApi.ts` wraps the Tauri settings
  commands.
- `GhwtdeIniTool/src/services/scanModsApi.ts` wraps the Tauri keep-pattern
  delete, `song.ini` scan/validation/disable/enable/conflict delete, and
  instrument analysis commands.
- `GhwtdeIniTool/src/services/modsFolderDialog.ts` wraps Tauri folder pickers.
- `GhwtdeIniTool/src/types/projectSettings.ts` defines the frontend settings
  shape returned by Rust.
- `GhwtdeIniTool/src/types/scanMods.ts` defines the frontend MODS scan,
  keep-pattern delete, and `song.ini` scan/validation/conflict/content result
  shapes.
- `GhwtdeIniTool/src/utils/keepOnlyFilesPattern.ts` defines the default
  keep-pattern and frontend validation helper.
- `GhwtdeIniTool/vite.config.ts` configures Vite for Tauri development on port
  `1420`.
- `GhwtdeIniTool/src-tauri/tauri.conf.json` configures the desktop shell,
  including the main window, bundling, and updater artifacts.

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
  keep-pattern, previews MODS files that do not match it, allows returning from
  preview to edit the pattern, and deletes only after the user finishes the
  review step. The pattern defaults to
  `song*.ini,*_song.pak.xen,*.fsb.xen,category*.ini,*.img.xen,Readme.txt`
  each time and is not saved.
- The Scan MODS folder button opens a three-step wizard that parses `song.ini`
  files, stores valid parsed results in backend memory, lets the user repair
  faulty files, then resolves duplicate checksum groups and folders containing
  both `song.ini` and `song.disabled.ini`. Faulty step-one entries can be
  disabled to rename `song.ini` to `song.disabled.ini`, then enabled again if
  needed. Duplicate checksums are resolved by disabling selected active songs;
  active/disabled sibling conflicts can delete either file.
  The final step validates each active song folder's `Content` and
  `Content/MUSIC` layout against the parsed checksum, accepts those folder and
  checksum-derived file names case-insensitively without renaming them, reports
  missing/misnamed/extra files, lets the user copy the native absolute path to
  the related song folder, and can disable that `song.ini`. Debug/Tauri dev runs
  temporarily suppress missing `Content/MUSIC` warnings when that folder is
  absent so local development can omit bulky MUSIC assets. The wizard scales
  with the app window while keeping preview content scrollable, and shows
  progress while finding `song.ini` files, reading songs, checking content, and
  finishing. Completion and errors are shown as compact toasts.
- The Analyze instruments top-bar button is enabled after a completed song scan
  and opens a separate wizard with `Read only missing`, `Reread only songs with
  errors`, and `Reread all` modes. It switches immediately to a progress step
  while PAK chart data is analyzed, writes `song.instruments.ini` sidecars next
  to active `song.ini` files, then updates the scanned-song table.
- After the Scan MODS folder wizard is finished, the main view shows a compact
  scanned-song table populated from parsed active `song.ini` files. It displays
  exact `[SongInfo]` `Artist`, `Title`, `Year`, `Genre`, and `GameIcon` values,
  plus Guitar, Bass, Drums, Vocals, CoopGuitar, and CoopBass availability read
  from each song's `song.instruments.ini` sidecar when that sidecar is fresh.
  Missing, stale, or invalid instrument sidecars display `Unknown`. Instrument
  cells show the highest available difficulty, expose all levels or analyzer
  errors through native hover tooltips, and participate in table sort/filter
  behavior. The table shows `<empty>` for missing display values so filters can
  find them, provides
  per-column filters, sortable/resizable data columns, editable metadata cells
  for the displayed `[SongInfo]` values, fixed-width row actions for copying the
  song folder, saving metadata edits, and restoring from `song.original.ini`
  when present.
- The settings dialog lets the user choose a MODS folder and toggle whether
  original `song.ini` files are kept before their first edit.
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
- `GhwtdeIniTool/src-tauri/src/song_pak_analyzer.rs` implements the read-only
  GHWT `*_song.pak.xen` analyzer used to detect playable instrument chart data
  from PAK/QB contents.
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
  `scan_song_ini_files`, `validate_song_ini_file`, `disable_song_ini_file`,
  `enable_song_ini_file`, and `delete_song_ini_conflict_file` Tauri commands.
  It also exposes `update_scanned_song_metadata` for row-level scanned-table edits and
  `restore_original_song_ini` for consuming a sibling `song.original.ini` back
  into the active `song.ini`.
- `scan_song_ini_files` emits `song_scan_progress` events while finding songs,
  reading `song.ini` files, checking content, and finishing.
- Exposes `analyze_scanned_song_instruments`, which analyzes instrument support
  for the current scanned song store in `missing`, `errors`, or `all` mode,
  runs the expensive PAK work on a blocking worker while emitting throttled
  progress events, writes `song.instruments.ini` sidecars beside active
  `song.ini` files, and returns refreshed table rows.
- Exposes `analyze_song_pak`, which takes a PAK path and song checksum, reads
  the matching main chart QB from the PAK without extracting files, and returns
  instrument support details, parser diagnostics, warnings, and errors in a
  frontend-ready serialized shape.
- Stores project settings in `ghwtdeinitool.ini` next to the executable under a
  `[project]` section.
- Reads and writes `mods_dir` and `keep_original_song_ini`. Missing
  `keep_original_song_ini` values default to `true`; legacy
  `keep_only_files_pattern` entries in old settings files are ignored.
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
  known `ModInfo`/`SongInfo` keys while allowing extra unknown keys, groups
  duplicate parsed `[SongInfo]` checksum values across valid active songs,
  reports sibling `song.ini`/`song.disabled.ini` conflicts, validates active song
  `Content` layouts against checksum-derived file names case-insensitively,
  reports strict extra files in `Content` and `Content/MUSIC`, accepts
  case-mismatched `Content`/`MUSIC` folders without renaming them, and validates
  repaired contents before writing them back to disk. Scan and validation
  results include a `songs` array for the frontend table, with each row's
  MODS-relative `song.ini` path, absolute song folder path, and display values
  parsed from exact `[SongInfo]` keys, plus whether a sibling
  `song.original.ini` is available for restore. Each scanned-song row also reads
  a fresh sibling `song.instruments.ini` sidecar for instrument availability, or
  returns `Unknown` instrument values when that sidecar is missing, stale, or
  invalid.
  Debug builds temporarily
  suppress missing `Content/MUSIC` folder warnings when that folder is absent so
  local development can omit bulky MUSIC assets. When
  `keep_original_song_ini` is enabled, the backend creates a sibling
  `song.original.ini` before the first write to each `song.ini` and before a
  `song.ini` is disabled by rename.
- Uses `tauri.conf.json` to open the main window maximized, restore it to
  1920x1080 when un-maximized, and configure bundling and updater artifacts.

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
  deletes, settings parsing/writing, `song.ini` parsing, faulty file reporting,
  validation writes, original backup behavior, store updates, duplicate key
  rejection, required `song.ini` fields, canonical key casing auto-correction,
  duplicate checksum grouping, disabled-file conflict reporting,
  disable/enable-by-rename behavior, conflict file deletion, content layout
  validation,
  content folder and checksum filename case-insensitive matching, debug-only
  missing `Content/MUSIC` suppression, strict extra content file reporting,
  refreshed content issues after checksum repair, song scan progress events,
  scanned-table metadata updates, original `song.ini` restore behavior,
  instrument summary display policy, instrument sidecar loading/staleness,
  strict instrument analysis mode selection, sidecar writes, case-insensitive
  song PAK resolution, and unsafe repair/action path rejection.
- Rust song PAK analyzer tests cover QBKey hashing, hash normalization, minimal
  PAK entry parsing, QB section parsing, instrument/vocal support detection, and
  an optional local parity check against the Sk8er Boi sample in `MODS_medium`
  when that folder exists.

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
