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
  cleanup wizard for keep-pattern entry plus sortable, selectable delete
  confirmation.
- `GhwtdeIniTool/src/components/CategorizeWizard.tsx` renders the two-step
  song categorization preview and confirmation workflow.
- `GhwtdeIniTool/src/components/ExperimentalWarningDialog.tsx` renders the
  read-only experimental-use disclaimer dialog.
- `GhwtdeIniTool/src/components/FixGameIconsWizard.tsx` renders the custom
  GameIcon category scan/fix wizard.
- `GhwtdeIniTool/src/components/InstrumentAnalyzeWizard.tsx` renders the
  separate instrument sidecar analysis wizard and progress state.
- `GhwtdeIniTool/src/components/RestoreIniWizard.tsx` renders the bulk
  `song.ini` backup restore dialog.
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
- `GhwtdeIniTool/src/hooks/useIniRestorer.ts` owns bulk `song.ini` restore
  dialog state.
- `GhwtdeIniTool/src/hooks/useModsScanner.ts` owns `song.ini` scan, step-one
  undo snapshots, instrument analysis, song categorization, and GameIcon
  category fixing state.
- `GhwtdeIniTool/src/hooks/useAppUpdater.ts` owns updater state and actions.
- `GhwtdeIniTool/src/services/projectSettingsApi.ts` wraps the Tauri settings
  commands.
- `GhwtdeIniTool/src/services/scanModsApi.ts` wraps the Tauri keep-pattern
  delete, `song.ini` scan/validation/step-one undo/disable/enable/conflict
  delete, instrument analysis, and GameIcon category commands.
- `GhwtdeIniTool/src/services/modsFolderDialog.ts` wraps Tauri folder pickers.
- `GhwtdeIniTool/src/types/projectSettings.ts` defines the frontend settings
  shape returned by Rust.
- `GhwtdeIniTool/src/types/scanMods.ts` defines the frontend MODS scan,
  keep-pattern delete, and `song.ini` scan/validation/conflict/content result
  shapes, plus GameIcon category scan result shapes.
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
- If no MODS folder or official GAMELOGOS folder is configured, either
  configured folder is unavailable, or the first-run disclaimer has not been
  accepted, settings status becomes `needs-folder` and the settings dialog
  opens automatically. The dialog cannot be closed until both required folders
  are available and the disclaimer is accepted.
- The settings button can reopen the dialog after startup.
- A persistent red experimental-risk warning appears beside Settings; selecting
  it opens a read-only dialog with the experimental-use disclaimer text.
- The Clean MODS folder button opens a two-step wizard that asks for a
  keep-pattern, previews MODS files that do not match it in sortable Path and
  File columns, allows individual or all previewed files to be deselected,
  allows returning from preview to edit the pattern, and deletes only the
  selected files after the user finishes the review step. The pattern defaults to
  `song*.ini,*_song.pak.xen,*.fsb.xen,category*.ini,*.img.xen,Readme.txt`
  each time and is not saved.
- The Restore INI files button opens a dialog with separate actions to restore
  active `song.ini` files or delete recursive `song.instruments.ini` sidecars.
  Restore actions first reactivate recursive `song.excluded.ini` files as
  `song.ini`, then use sibling backups with plain file copy/delete operations,
  without parsing backup contents. The default restore action restores
  `song.original.ini` backups and leaves `song.original.faulty.ini` in place.
  The pre-format-fix mode first consumes normal `song.original.ini` backups,
  then consumes `song.original.faulty.ini` backups so the pre-fix contents
  overwrite the working restore. Restoring clears any completed scan table so
  stale parsed metadata is not shown. If any files fail to restore, the dialog
  stays open and lists the backend restore errors.
- The Scan MODS folder button opens a three-step wizard that parses `song.ini`
  and `song.excluded.ini` files, stores valid parsed results in backend memory,
  lets the user repair
  faulty files, then resolves duplicate checksum groups and folders containing
  multiple `song.ini`, `song.excluded.ini`, or `song.disabled.ini` variants.
  Faulty step-one entries can be
  edited and repaired with their pre-repair contents preserved as
  `song.original.faulty.ini`, undone after the first edit or repair while the
  wizard remains open by writing the scan-time faulty contents back from
  frontend memory, or disabled to rename `song.ini` to `song.disabled.ini`,
  then enabled again if needed; excluded-origin files return to
  `song.excluded.ini` when re-enabled. Duplicate checksums are resolved by
  toggling selected songs between `song.ini` and `song.excluded.ini`; same-folder INI conflicts
  list every present variant and can delete files until one remains.
  The final step validates each active song folder's `Content` and
  `Content/MUSIC` layout against the parsed checksum, accepts those folder and
  checksum-derived file names case-insensitively without renaming them, reports
  missing/misnamed/extra files, requires missing files to be resolved before
  the scan can finish, lists extra files without blocking completion, lets the
  user copy the native absolute path to the related song folder, verify all
  affected songs together after external file fixes or folder deletions, and
  can disable that `song.ini`. Debug/Tauri dev runs
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
- The Fix GameIcons top-bar button is enabled after a completed song scan and
  opens a two-step GameIcon wizard. Step 1 scans MODS for
  case-insensitive `gamelogo_*.img.xen` files, groups them by folder, shows a
  `category.ini` chip only when the sibling `[CategoryInfo] Logo` matches the
  gamelogo stem, and can fix folders by moving multiple gamelogos into
  `Category_<gamelogo_stem>` subfolders and generating or replacing
  `category.ini` files. Step 2 previews songs whose `GameIcon` is not known
  from official or custom gamelogos, shows backend-guessed replacements based
  on valid siblings or matching custom categories in the songs' parent folder,
  displays each song's MODS-relative parent folder, lets only the replacement
  value be edited or cleared, and can bulk-fill the pending replacements for
  listed invalid sibling songs before applying the final batch to `song.ini`
  files. Clearing a value removes the `GameIcon` entry entirely. The
  categorization-generated top-level `IniToolCategories` folder is excluded
  from custom GameIcon discovery.
- After the Scan MODS folder wizard is finished, the main view shows a compact
  scanned-song table populated from parsed active `song.ini` files. It displays
  exact `[SongInfo]` `Artist`, `Title`, `Year`, `Genre`, and `GameIcon` values,
  plus Guitar, Bass, Drums, Vocals, CoopGuitar, and CoopBass availability read
  from each song's `song.instruments.ini` sidecar when that sidecar is fresh.
  Missing, stale, or invalid instrument sidecars display `Unknown`. Instrument
  cells show the highest available difficulty, expose all levels or analyzer
  errors through native hover tooltips, and participate in table sort/filter
  behavior. The table shows `<empty>` for missing display values so filters can
  find them, provides per-column filters with scanned-value dropdowns for Year,
  Genre, GameIcon, and instrument columns, sortable/resizable data columns, an
  Include checkbox column with `All`, `Included`, and `Excluded` filtering for
  choosing which songs remain eligible for later processing. Songs read from
  `song.excluded.ini` start unchecked; toggling alone does not rename files,
  while excluding a duplicate in the scan wizard, saving metadata, or restoring
  a row writes the selected active/excluded filename. The table provides
  file-like row
  selection with a right-click include/exclude selection menu, editable metadata
  cells for the displayed `[SongInfo]` values,
  fixed-width row actions for copying the song folder, saving metadata edits,
  and restoring from a proper `song.original.ini` when present. Faulty
  `song.original.faulty.ini` backups are not restorable from the table. The
  table header shows included, visible, and total row counts, duplicate checksum
  rows are faintly highlighted, and re-including multiple songs from the same
  duplicate group reopens the duplicate resolver directly on the conflict step.
  The top action row includes Categorize alongside Analyze instruments and Fix
  GameIcons. Categorize currently opens only its first informational step: it
  reports the table sort order, active filters, unsaved metadata edits, the
  selected-song cap, the projected 200-song categories, and whether the
  existing `IniToolCategories` folder contains files other than `category.ini`.
  Its Next button is disabled until the later filesystem categorization step is
  implemented.
- The settings dialog lets the user choose a MODS folder, choose an official
  GAMELOGOS folder after MODS is set, toggle whether original `song.ini` files
  are kept before their first edit, and accept a one-time experimental-use
  disclaimer before workflows can be used.
- Scanned-song saves require non-empty Artist and Title values. Empty Year,
  Genre, or GameIcon values remove their `[SongInfo]` entries rather than
  writing empty `Key=` lines; the shared INI writer applies that omission rule
  to any empty update value.
- When a MODS folder is saved, settings try to auto-fill the official GAMELOGOS
  folder by replacing each exact `MODS` path component with `IMAGES`, nearest
  first, appending `GAMELOGOS`, dropping folders below the matched `MODS`
  component, and using the first existing directory.
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
  `scan_song_ini_files`, `validate_song_ini_file`, `undo_song_ini_repair`,
  `verify_song_ini_file`, `disable_song_ini_file`, `enable_song_ini_file`, and
  `delete_song_ini_conflict_file` Tauri commands.
  It also exposes `update_scanned_song_metadata` for row-level scanned-table edits and
  `restore_original_song_ini` for consuming a sibling `song.original.ini` back
  into the active `song.ini`, plus `restore_all_original_song_ini` for bulk
  restore from normal or pre-format-fix backups.
- `scan_song_ini_files` emits `song_scan_progress` events while finding songs,
  reading `song.ini` files, checking content, and finishing.
- Exposes `analyze_scanned_song_instruments`, which analyzes instrument support
  for the current scanned song store in `missing`, `errors`, or `all` mode,
  runs the expensive PAK work on a blocking worker while emitting throttled
  progress events, writes `song.instruments.ini` sidecars beside active
  `song.ini` files, and returns refreshed table rows.
- Exposes `scan_game_icon_categories` and `fix_game_icon_categories` for the
  GameIcon wizard. These commands recursively scan MODS for case-insensitive
  `gamelogo_*.img.xen` files, excluding the top-level `IniToolCategories`
  output folder, parse sibling `category.ini` files with duplicate
  key rejection, compare `[CategoryInfo] Logo` to the gamelogo stem, return
  original and custom gamelogo stems without `.img.xen`, and fix custom
  categories by splitting multi-gamelogo folders, renaming invalid or
  mismatched `category.ini` files to `category.original.faulty.ini`, and
  writing generated category metadata.
- Exposes `preview_game_icon_song_fixes` and `apply_game_icon_song_fixes` for
  the GameIcon wizard's second step. The preview command uses the current
  scanned song store plus official/custom gamelogo stems to return invalid
  song rows with guessed replacements. The apply command validates replacement
  stems, updates only `GameIcon` in each target `song.ini`, preserves existing
  backup behavior, and returns refreshed scanned-song table data.
- Exposes `analyze_song_pak`, which takes a PAK path and song checksum, reads
  the matching main chart QB from the PAK without extracting files, trying
  `songs/<checksum>` QB names before root-level GHWT-style names, then falls
  back to checksum-specific QB section discovery when the entry filename hash
  uses an unknown convention. It returns instrument support details, parser
  diagnostics, warnings, errors, the lookup strategy, selected entry hash,
  matching section count, and matched QB filename when known in a
  frontend-ready serialized shape.
- Stores project settings in `ghwtdeinitool.ini` next to the executable under a
  `[project]` section. Windows folder paths use conventional readable forms
  such as `C:\Games\GHWT\DATA\MODS`, without the `\\?\` prefix or escaped
  backslashes.
- Reads and writes `mods_dir`, `official_gamelogos_dir`, and
  `keep_original_song_ini`. Missing `official_gamelogos_dir` values default to
  unset, missing `keep_original_song_ini` values default to `true`, and legacy
  `keep_only_files_pattern` entries in old settings files are ignored.
- Reports `mods_dir_available` and `official_gamelogos_dir_available` by
  checking whether each stored path still exists as a directory.
- Requires `mods_dir` to be an existing directory before saving settings.
  `official_gamelogos_dir` is stored only when it exists or can be derived;
  otherwise setup remains incomplete until the user chooses it manually.
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
  `song.original.faulty.ini` before writing repaired wizard step-one files, and
  creates a sibling `song.original.ini` before scanned-table metadata edits or
  before a `song.ini` is disabled by rename. Step-one Undo uses the frontend's
  scan-time faulty contents snapshot while the wizard is open, so it does not
  depend on `song.original.faulty.ini`. Only `song.original.ini` marks a row as
  restorable and can be consumed by row-level restore; the bulk restore dialog's
  pre-format-fix mode can also consume `song.original.faulty.ini` after normal
  backups have been restored.
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
  refreshed content issues after checksum repair or verify, song scan progress events,
  scanned-table metadata updates, row-level and bulk original `song.ini`
  restore behavior, instrument summary display policy, instrument sidecar loading/staleness,
  strict instrument analysis mode selection, sidecar writes, case-insensitive
  song PAK resolution, unsafe repair/action path rejection, GameIcon category
  discovery/fixing, invalid category backup/regeneration, collision handling,
  original/custom gamelogo stem listing, invalid song GameIcon preview
  guessing, and song GameIcon apply validation/writes.
- Rust song PAK analyzer tests cover QBKey hashing, hash normalization, minimal
  PAK entry parsing, `songs/`-path precedence, root-level GHWT-style QB lookup,
  checksum-specific structural QB discovery and ranking, QB section parsing,
  instrument/vocal support detection, and an optional local parity check
  against the Sk8er Boi sample in `MODS_medium` when that folder exists.

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
