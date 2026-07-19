use ab_glyph::{FontArc, PxScale};
use image::{ImageFormat, Rgba, RgbaImage};
use imageproc::drawing::{draw_text_mut, text_size};
use ini::Ini;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};

mod song_pak_analyzer;

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
const INSTRUMENT_SIDECAR_FILE_NAME: &str = "song.instruments.ini";
const INSTRUMENT_ANALYZER_VERSION: &str = "1";
const CATEGORY_LOGO_SIZE: u32 = 256;
const CATEGORY_LOGO_STROKE_WIDTH: i32 = 3;
const CATEGORY_LOGO_MAX_LINE_WIDTH: u32 = 228;
const CATEGORY_LOGO_MAX_LINE_HEIGHT: u32 = 68;
const CATEGORY_LOGO_LINE_GAP: u32 = 6;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const IMG_XEN_HEADER_SIZE: usize = 40;
const THIRD_PARTY_LICENSES: &str =
    include_str!(concat!(env!("OUT_DIR"), "/third_party_licenses.txt"));
const MOD_INFO_KEYS: &[&str] = &["Key", "Name", "Description", "Author", "Version"];
const SONG_INFO_KEYS: &[&str] = &[
    "Key",
    "Checksum",
    "Title",
    "ArtistText",
    "Artist",
    "Year",
    "CoverArtist",
    "CoverYear",
    "OriginalArtist",
    "Leaderboard",
    "Singer",
    "Genre",
    "Countoff",
    "Drumkit",
    "Volume",
    "GameIcon",
    "GameCategory",
    "Band",
    "UseNewClips",
    "SkeletonTypeG",
    "SkeletonTypeB",
    "SkeletonTypeD",
    "SkeletonTypeV",
    "MicForGuitarist",
    "MicForBassist",
    "SongLength",
    "IntroTheme",
];

#[derive(Serialize)]
struct ProjectSettings {
    disclaimer_accepted: bool,
    mods_dir: Option<String>,
    mods_dir_available: bool,
    official_gamelogos_dir: Option<String>,
    official_gamelogos_dir_available: bool,
    keep_original_song_ini: bool,
    check_for_updates_on_startup: bool,
    settings_file: String,
}

#[derive(Deserialize)]
struct ProjectSettingsInput {
    disclaimer_accepted: Option<bool>,
    mods_dir: Option<String>,
    official_gamelogos_dir: Option<String>,
    keep_original_song_ini: Option<bool>,
    check_for_updates_on_startup: Option<bool>,
}

#[derive(Default, Serialize)]
struct DeleteFilesPreview {
    files_to_delete: Vec<String>,
    errors: Vec<String>,
}

#[derive(Default, Serialize)]
struct DeleteFilesResult {
    files_deleted: usize,
    errors: Vec<String>,
}

#[derive(Default, Serialize)]
struct IniToolCategoriesPreview {
    has_non_category_files: bool,
}

#[derive(Deserialize)]
struct CategorizeSongsInput {
    ordered_song_paths: Vec<String>,
    included_song_paths: Vec<String>,
    maximum_song_cap: usize,
    category_names: Vec<String>,
}

#[derive(Serialize)]
struct CategorizeSongsResult {
    categorized: usize,
    excluded: usize,
    songs_parsed: usize,
    songs: Vec<ScannedSong>,
    duplicate_checksum_groups: Vec<DuplicateChecksumGroup>,
    song_ini_folder_conflicts: Vec<SongIniFolderConflict>,
    content_file_issues: Vec<SongContentIssue>,
}

#[derive(Default, Serialize)]
struct RestoreIniResult {
    files_restored: usize,
    files_deleted: usize,
    errors: Vec<String>,
}

#[derive(Default)]
struct SongIniStore(Mutex<Vec<ParsedSongIni>>);

struct SettingsStore {
    path: PathBuf,
}

#[derive(Clone, Serialize)]
struct ParsedSongIni {
    relative_path: String,
    sections: Vec<ParsedIniSection>,
}

#[derive(Clone, Serialize)]
struct ParsedIniSection {
    name: Option<String>,
    entries: Vec<ParsedIniEntry>,
}

#[derive(Clone, Serialize)]
struct ParsedIniEntry {
    key: String,
    value: String,
}

#[derive(Clone, Deserialize)]
struct ScannedSongMetadataInput {
    artist: String,
    title: String,
    year: String,
    genre: String,
    game_icon: String,
}

#[derive(Default, Serialize)]
struct SongIniScanResult {
    songs_found: usize,
    songs_parsed: usize,
    songs: Vec<ScannedSong>,
    faulty_files: Vec<FaultySongIniFile>,
    duplicate_checksum_groups: Vec<DuplicateChecksumGroup>,
    song_ini_folder_conflicts: Vec<SongIniFolderConflict>,
    content_file_issues: Vec<SongContentIssue>,
    errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ScannedSong {
    relative_path: String,
    folder_absolute_path: String,
    artist: String,
    title: String,
    year: String,
    genre: String,
    game_icon: String,
    has_original_song_ini: bool,
    is_included: bool,
    instruments: ScannedSongInstruments,
}

#[derive(Clone, Debug, Serialize)]
struct ScannedSongInstruments {
    guitar: InstrumentColumnSummary,
    bass: InstrumentColumnSummary,
    drums: InstrumentColumnSummary,
    vocals: InstrumentColumnSummary,
    coop_guitar: InstrumentColumnSummary,
    coop_bass: InstrumentColumnSummary,
}

#[derive(Clone, Debug, Serialize)]
struct InstrumentColumnSummary {
    value: String,
    tooltip: String,
    easy: bool,
    medium: bool,
    hard: bool,
    expert: bool,
    errors: Vec<String>,
}

#[derive(Clone, Debug)]
struct PakIdentity {
    status: String,
    relative_path: String,
    size: Option<u64>,
    modified_millis: Option<u128>,
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct SidecarReadResult {
    instruments: Option<ScannedSongInstruments>,
    has_fresh_error: bool,
}

#[derive(Clone, Serialize)]
struct FaultySongIniFile {
    relative_path: String,
    contents: String,
    error: String,
}

#[derive(Clone, Debug, Serialize)]
struct DuplicateChecksumGroup {
    checksum: String,
    relative_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SongIniFolderConflict {
    folder_path: String,
    file_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SongContentIssue {
    song_ini_relative_path: String,
    song_ini_absolute_path: String,
    checksum: String,
    message: String,
    absolute_path: String,
}

#[derive(Debug, Serialize)]
struct SongIniValidationResult {
    relative_path: String,
    contents: String,
    songs_parsed: usize,
    songs: Vec<ScannedSong>,
    duplicate_checksum_groups: Vec<DuplicateChecksumGroup>,
    song_ini_folder_conflicts: Vec<SongIniFolderConflict>,
    content_file_issues: Vec<SongContentIssue>,
}

#[derive(Debug, Serialize)]
struct SongContentVerificationResult {
    songs_parsed: usize,
    songs: Vec<ScannedSong>,
    duplicate_checksum_groups: Vec<DuplicateChecksumGroup>,
    song_ini_folder_conflicts: Vec<SongIniFolderConflict>,
    content_file_issues: Vec<SongContentIssue>,
}

#[derive(Debug, Serialize)]
struct SongIniDisableResult {
    relative_path: String,
    disabled_path: String,
    is_included: bool,
    songs_parsed: usize,
}

#[derive(Debug, Serialize)]
struct SongIniEnableResult {
    relative_path: String,
    enabled_path: String,
    is_included: bool,
    songs_parsed: usize,
}

#[derive(Debug, Serialize)]
struct SongIniDeleteResult {
    relative_path: String,
    songs_parsed: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum RestoreIniAction {
    AfterFormatIssueFixes,
    BeforeFormatIssueFix,
    DeleteInstrumentSidecars,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum InstrumentAnalyzeMode {
    Missing,
    Errors,
    All,
}

#[derive(Debug, Serialize)]
struct InstrumentAnalyzeResult {
    songs: Vec<ScannedSong>,
    analyzed: usize,
    skipped: usize,
    errors: usize,
}

#[derive(Clone, Debug, Serialize)]
struct InstrumentAnalyzeProgress {
    current: usize,
    total: usize,
    relative_path: String,
    mode: String,
}

#[derive(Clone, Debug, Serialize)]
struct GameIconCategoryScanResult {
    groups: Vec<GameIconCategoryGroup>,
    needs_fix: bool,
    official_game_icons: Vec<String>,
    custom_game_icons: Vec<String>,
    errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GameIconCategoryGroup {
    folder_relative_path: String,
    folder_absolute_path: String,
    gamelogos: Vec<GameIconFile>,
    has_multiple_gamelogos: bool,
}

#[derive(Clone, Debug, Serialize)]
struct GameIconFile {
    relative_path: String,
    file_name: String,
    stem: String,
    has_matching_category_ini: bool,
}

#[derive(Clone, Debug, Serialize)]
struct GameIconSongFixPreview {
    rows: Vec<GameIconSongFixRow>,
    valid_game_icons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GameIconSongFixRow {
    relative_path: String,
    parent_relative_path: String,
    artist: String,
    title: String,
    invalid_game_icon: String,
    new_game_icon: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GameIconSongFixInput {
    relative_path: String,
    new_game_icon: String,
}

#[derive(Debug, Serialize)]
struct GameIconSongFixApplyResult {
    applied: usize,
    songs_parsed: usize,
    songs: Vec<ScannedSong>,
    duplicate_checksum_groups: Vec<DuplicateChecksumGroup>,
    song_ini_folder_conflicts: Vec<SongIniFolderConflict>,
    content_file_issues: Vec<SongContentIssue>,
}

#[derive(Clone, Debug)]
struct CustomGameIconLocation {
    stem: String,
    folder_relative_path: String,
    has_matching_category_ini: bool,
}

#[derive(Clone, Debug, Serialize)]
struct SongScanProgress {
    phase: String,
    current: usize,
    total: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    relative_path: String,
}

struct StoredProjectSettings {
    disclaimer_accepted: bool,
    mods_dir: Option<String>,
    official_gamelogos_dir: Option<String>,
    keep_original_song_ini: bool,
    check_for_updates_on_startup: bool,
}

#[tauri::command]
fn load_project_settings(
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<ProjectSettings, String> {
    let settings_path = settings_store.path.clone();
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    Ok(project_settings(settings_path, settings))
}

#[tauri::command]
fn save_project_settings(
    settings: ProjectSettingsInput,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<ProjectSettings, String> {
    let settings_path = settings_store.path.clone();
    let previous_settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };
    let settings = validate_project_settings(settings, &previous_settings)?;
    let contents = write_project_settings_to_ini(&settings);

    if let Some(settings_dir) = settings_path.parent() {
        fs::create_dir_all(settings_dir).map_err(|err| {
            format!(
                "Failed to create settings directory at {}: {err}",
                settings_dir.display()
            )
        })?;
    }

    fs::write(&settings_path, contents).map_err(|err| {
        format!(
            "Failed to write settings file at {}: {err}",
            settings_path.display()
        )
    })?;

    Ok(project_settings(settings_path, settings))
}

#[tauri::command]
fn third_party_licenses() -> String {
    THIRD_PARTY_LICENSES.to_string()
}

#[tauri::command]
fn preview_keep_only_files_delete(
    keep_only_files_pattern: String,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<DeleteFilesPreview, String> {
    let settings = scan_settings(&settings_store.path)?;
    preview_keep_only_files_delete_paths(&settings.mods_dir, &keep_only_files_pattern)
}

#[tauri::command]
fn delete_keep_only_files(
    files_to_delete: Vec<String>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<DeleteFilesResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    delete_keep_only_files_paths(&settings.mods_dir, &files_to_delete)
}

#[tauri::command]
fn preview_ini_tool_categories(
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<IniToolCategoriesPreview, String> {
    let settings = scan_settings(&settings_store.path)?;
    preview_ini_tool_categories_paths(&settings.mods_dir)
}

#[tauri::command]
fn categorize_songs(
    input: CategorizeSongsInput,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<CategorizeSongsResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    categorize_songs_paths(&settings, input, &store)
}

#[tauri::command]
fn scan_song_ini_files(
    app: tauri::AppHandle,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniScanResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    scan_song_ini_files_paths_with_progress(&settings, &store, |progress| {
        let _ = app.emit("song_scan_progress", progress);
    })
}

#[tauri::command]
fn validate_song_ini_file(
    relative_path: String,
    contents: String,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    validate_song_ini_file_path(&settings, &relative_path, &contents, &store)
}

#[tauri::command]
fn undo_song_ini_repair(
    relative_path: String,
    contents: String,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    undo_song_ini_repair_path(&settings, &relative_path, &contents, &store)
}

#[tauri::command]
fn verify_content_issue_songs(
    relative_paths: Vec<String>,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongContentVerificationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    verify_content_issue_songs_paths(&settings, &relative_paths, &store)
}

#[tauri::command]
fn disable_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniDisableResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    disable_song_ini_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn enable_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniEnableResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    enable_song_ini_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn delete_song_ini_conflict_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniDeleteResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    delete_song_ini_conflict_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn set_scanned_song_included(
    relative_path: String,
    is_included: bool,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    set_scanned_song_included_path(&settings, &relative_path, is_included, &store)
}

#[tauri::command]
fn update_scanned_song_metadata(
    relative_path: String,
    metadata: ScannedSongMetadataInput,
    is_included: bool,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    update_scanned_song_metadata_path(&settings, &relative_path, metadata, is_included, &store)
}

#[tauri::command]
fn restore_original_song_ini(
    relative_path: String,
    is_included: bool,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    restore_original_song_ini_path(&settings, &relative_path, is_included, &store)
}

#[tauri::command]
fn restore_all_original_song_ini(
    action: RestoreIniAction,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<RestoreIniResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    restore_all_ini_paths(&settings, action, &store)
}

#[tauri::command]
fn scan_game_icon_categories(
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<GameIconCategoryScanResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    scan_game_icon_categories_paths(&settings)
}

#[tauri::command]
fn fix_game_icon_categories(
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<GameIconCategoryScanResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    fix_game_icon_categories_paths(&settings)
}

#[tauri::command]
fn preview_game_icon_song_fixes(
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<GameIconSongFixPreview, String> {
    let settings = scan_settings(&settings_store.path)?;
    preview_game_icon_song_fixes_paths(&settings, &store)
}

#[tauri::command]
fn apply_game_icon_song_fixes(
    fixes: Vec<GameIconSongFixInput>,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<GameIconSongFixApplyResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    apply_game_icon_song_fixes_paths(&settings, fixes, &store)
}

#[tauri::command]
async fn analyze_scanned_song_instruments(
    app: tauri::AppHandle,
    mode: InstrumentAnalyzeMode,
    store: tauri::State<'_, SongIniStore>,
    settings_store: tauri::State<'_, SettingsStore>,
) -> Result<InstrumentAnalyzeResult, String> {
    let settings = scan_settings(&settings_store.path)?;
    let parsed_songs = stored_parsed_songs(&store)?;
    drop(store);

    tauri::async_runtime::spawn_blocking(move || {
        analyze_scanned_song_instruments_for_songs(&settings, mode, parsed_songs, |progress| {
            let _ = app.emit("instrument_scan_progress", progress);
        })
    })
    .await
    .map_err(|err| format!("Instrument analysis task failed: {err}"))?
}

#[tauri::command]
fn analyze_song_pak(
    pak_path: String,
    checksum: String,
) -> Result<song_pak_analyzer::SongPakAnalysis, String> {
    song_pak_analyzer::analyze_song_pak(&pak_path, &checksum)
}

struct ScanSettings {
    mods_dir: PathBuf,
    official_gamelogos_dir: Option<PathBuf>,
    keep_original_song_ini: bool,
}

fn scan_settings(settings_path: &Path) -> Result<ScanSettings, String> {
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;

    Ok(ScanSettings {
        mods_dir,
        official_gamelogos_dir: existing_optional_dir(settings.official_gamelogos_dir),
        keep_original_song_ini: settings.keep_original_song_ini,
    })
}

fn project_settings(settings_path: PathBuf, settings: StoredProjectSettings) -> ProjectSettings {
    let mods_dir_available = settings
        .mods_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());
    let official_gamelogos_dir_available = settings
        .official_gamelogos_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());

    ProjectSettings {
        disclaimer_accepted: settings.disclaimer_accepted,
        mods_dir: settings.mods_dir,
        mods_dir_available,
        official_gamelogos_dir: settings.official_gamelogos_dir,
        official_gamelogos_dir_available,
        keep_original_song_ini: settings.keep_original_song_ini,
        check_for_updates_on_startup: settings.check_for_updates_on_startup,
        settings_file: settings_path.to_string_lossy().into_owned(),
    }
}

fn preview_keep_only_files_delete_paths(
    mods_dir: &Path,
    keep_only_files_pattern: &str,
) -> Result<DeleteFilesPreview, String> {
    let keep_patterns = keep_patterns(keep_only_files_pattern);

    if keep_patterns.is_empty() {
        return Ok(DeleteFilesPreview::default());
    }

    let mut preview = DeleteFilesPreview::default();
    collect_files_to_delete(mods_dir, mods_dir, &keep_patterns, &mut preview)?;
    Ok(preview)
}

fn collect_files_to_delete(
    mods_dir: &Path,
    dir: &Path,
    keep_patterns: &[String],
    preview: &mut DeleteFilesPreview,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            collect_files_to_delete(mods_dir, &path, keep_patterns, preview)?;
        } else if file_type.is_file() && !matches_keep_patterns(&path, keep_patterns) {
            preview
                .files_to_delete
                .push(mods_relative_path(mods_dir, &path)?);
        }
    }

    Ok(())
}

fn delete_keep_only_files_paths(
    mods_dir: &Path,
    files_to_delete: &[String],
) -> Result<DeleteFilesResult, String> {
    let mut result = DeleteFilesResult::default();

    for relative_path in files_to_delete {
        let path = match checked_mods_relative_path(mods_dir, relative_path) {
            Ok(path) => path,
            Err(err) => {
                result.errors.push(err);
                continue;
            }
        };

        match fs::remove_file(&path) {
            Ok(()) => result.files_deleted += 1,
            Err(err) => result
                .errors
                .push(format!("Failed to delete {}: {err}", path.display())),
        }
    }

    Ok(result)
}

fn preview_ini_tool_categories_paths(mods_dir: &Path) -> Result<IniToolCategoriesPreview, String> {
    let categories_dir = mods_dir.join("IniToolCategories");

    if !categories_dir.exists() {
        return Ok(IniToolCategoriesPreview::default());
    }

    if !categories_dir.is_dir() {
        return Err(format!(
            "{} exists but is not a folder.",
            categories_dir.display()
        ));
    }

    Ok(IniToolCategoriesPreview {
        has_non_category_files: contains_non_category_file(&categories_dir)?,
    })
}

fn contains_non_category_file(dir: &Path) -> Result<bool, String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            if contains_non_category_file(&path)? {
                return Ok(true);
            }
        } else if file_type.is_file()
            && !path
                .file_name()
                .is_some_and(|file_name| file_name.eq_ignore_ascii_case("category.ini"))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn categorize_songs_paths(
    settings: &ScanSettings,
    input: CategorizeSongsInput,
    store: &SongIniStore,
) -> Result<CategorizeSongsResult, String> {
    const SONGS_PER_CATEGORY: usize = 200;
    const MAXIMUM_SONG_CAP: usize = 19_800;

    if input.maximum_song_cap == 0 || input.maximum_song_cap > MAXIMUM_SONG_CAP {
        return Err("Enter a whole number from 1 through 19800.".to_string());
    }

    let stored_songs = stored_parsed_songs(store)?;
    let stored_paths = stored_songs
        .iter()
        .map(|song| song.relative_path.as_str())
        .collect::<HashSet<_>>();
    let ordered_paths = input.ordered_song_paths.iter().collect::<HashSet<_>>();
    if ordered_paths.len() != input.ordered_song_paths.len()
        || ordered_paths.len() != stored_paths.len()
        || !ordered_paths
            .iter()
            .all(|path| stored_paths.contains(path.as_str()))
    {
        return Err(
            "The scanned song order is stale. Scan MODS again before categorizing.".to_string(),
        );
    }

    let included_paths = input.included_song_paths.iter().collect::<HashSet<_>>();
    if included_paths.len() != input.included_song_paths.len()
        || !included_paths
            .iter()
            .all(|path| stored_paths.contains(path.as_str()))
    {
        return Err(
            "The included song list is stale. Scan MODS again before categorizing.".to_string(),
        );
    }

    let categorized_paths = input
        .ordered_song_paths
        .iter()
        .filter(|path| included_paths.contains(path))
        .take(input.maximum_song_cap)
        .cloned()
        .collect::<Vec<_>>();
    let category_count = categorized_paths.len().div_ceil(SONGS_PER_CATEGORY);
    if input.category_names.len() != category_count {
        return Err("The category preview is stale. Return to step 1 and try again.".to_string());
    }

    let category_logo_font = load_category_logo_font()?;

    let categories_dir = settings.mods_dir.join("IniToolCategories");
    if categories_dir.exists() && !categories_dir.is_dir() {
        return Err(format!(
            "{} exists but is not a folder.",
            categories_dir.display()
        ));
    }
    if categories_dir.exists() {
        for entry in fs::read_dir(&categories_dir)
            .map_err(|err| format!("Failed to read {}: {err}", categories_dir.display()))?
        {
            let path = entry
                .map_err(|err| {
                    format!(
                        "Failed to read entry in {}: {err}",
                        categories_dir.display()
                    )
                })?
                .path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            }
            .map_err(|err| format!("Failed to delete {}: {err}", path.display()))?;
        }
    } else {
        fs::create_dir_all(&categories_dir)
            .map_err(|err| format!("Failed to create {}: {err}", categories_dir.display()))?;
    }

    for (index, category_name) in input.category_names.iter().enumerate() {
        let category_number = index + 1;
        let checksum = format!("IniToolCategory{category_number:02}");
        let logo_stem = format!("gamelogo_initool{category_number:02}");
        let folder = categories_dir.join(format!(
            "Category_{}",
            sanitize_category_folder_name(category_name)
        ));
        fs::create_dir(&folder)
            .map_err(|err| format!("Failed to create {}: {err}", folder.display()))?;
        let contents = format!(
            "[ModInfo]\nName=IniTool Category {category_number:02}\nDescription=Songs categorized by {category_name}.\nAuthor=GhwtDeIniTool\nVersion=1.0\n\n[CategoryInfo]\nName={category_name}\nChecksum={checksum}\nLogo={logo_stem}\n"
        );
        let category_ini = folder.join("category.ini");
        fs::write(&category_ini, contents)
            .map_err(|err| format!("Failed to write {}: {err}", category_ini.display()))?;
        write_category_logo(&folder, category_number, category_name, &category_logo_font)?;
    }

    let categorized_path_set = categorized_paths.iter().collect::<HashSet<_>>();
    for song in stored_songs {
        let path = checked_mods_relative_path(&settings.mods_dir, &song.relative_path)?;
        let is_categorized = categorized_path_set.contains(&song.relative_path);
        let target_path = song_ini_path_for_inclusion(&path, is_categorized);
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let updated_contents = if is_categorized {
            let category_index = categorized_paths
                .iter()
                .position(|candidate| candidate == &song.relative_path)
                .expect("categorized song path must exist")
                / SONGS_PER_CATEGORY;
            let category_checksum = format!("IniToolCategory{:02}", category_index + 1);
            update_song_ini_values(
                &normalize_song_ini_key_case(&contents),
                &[("GameCategory", category_checksum.as_str())],
            )
        } else {
            normalize_song_ini_key_case(&contents)
        };
        write_scanned_song_ini_file(
            &path,
            &target_path,
            &updated_contents,
            settings.keep_original_song_ini,
        )?;
    }

    let scan = scan_song_ini_files_paths_with_progress(settings, store, |_| {})?;
    Ok(CategorizeSongsResult {
        categorized: categorized_paths.len(),
        excluded: scan.songs.len() - categorized_paths.len(),
        songs_parsed: scan.songs_parsed,
        songs: scan.songs,
        duplicate_checksum_groups: scan.duplicate_checksum_groups,
        song_ini_folder_conflicts: scan.song_ini_folder_conflicts,
        content_file_issues: scan.content_file_issues,
    })
}

fn sanitize_category_folder_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    character,
                    '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
        .fold(String::new(), |mut output, character| {
            if character.is_whitespace() {
                if !output.ends_with('_') {
                    output.push('_');
                }
            } else {
                output.push(character);
            }
            output
        });
    let sanitized = sanitized.trim_matches(|character: char| {
        character == '_' || character == '.' || character.is_whitespace()
    });

    if sanitized.is_empty() {
        "empty".to_string()
    } else {
        sanitized.to_string()
    }
}

fn load_category_logo_font() -> Result<FontArc, String> {
    FontArc::try_from_slice(include_bytes!("../resources/fonts/NewRocker-Regular.ttf"))
        .map_err(|err| format!("Failed to load bundled New Rocker category-logo font: {err}"))
}

fn write_category_logo(
    folder: &Path,
    category_number: usize,
    category_name: &str,
    font: &FontArc,
) -> Result<(), String> {
    let lines = category_logo_lines(category_number, category_name);
    let png = encode_category_logo_png(&lines, font)?;
    let img_xen = png_to_img_xen(&png)?;
    let logo_path = folder.join(format!("gamelogo_initool{category_number:02}.img.xen"));
    fs::write(&logo_path, img_xen)
        .map_err(|err| format!("Failed to write {}: {err}", logo_path.display()))?;

    Ok(())
}

fn encode_category_logo_png(lines: &[String; 3], font: &FontArc) -> Result<Vec<u8>, String> {
    let mut png = io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(render_category_logo(lines, font))
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|err| format!("Failed to encode category logo PNG: {err}"))?;
    Ok(png.into_inner())
}

fn png_to_img_xen(png: &[u8]) -> Result<Vec<u8>, String> {
    if png.len() < 33 || png[..8] != PNG_SIGNATURE {
        return Err("Generated category logo is not a valid PNG.".to_string());
    }

    let ihdr_length = u32::from_be_bytes(png[8..12].try_into().expect("IHDR length bytes"));
    if ihdr_length != 13 || &png[12..16] != b"IHDR" {
        return Err("Generated category logo PNG has no valid IHDR chunk.".to_string());
    }

    let width = u32::from_be_bytes(png[16..20].try_into().expect("PNG width bytes"));
    let height = u32::from_be_bytes(png[20..24].try_into().expect("PNG height bytes"));
    if width == 0 || width > u16::MAX as u32 || height == 0 || height > u16::MAX as u32 {
        return Err(format!(
            "Generated category logo dimensions {width}x{height} are unsupported."
        ));
    }
    let payload_length = u32::try_from(png.len())
        .map_err(|_| "Generated category logo PNG is too large.".to_string())?;

    let mut header = [0_u8; IMG_XEN_HEADER_SIZE];
    header[0..4].copy_from_slice(&0x0a28_1300_u32.to_be_bytes());
    header[8..10].copy_from_slice(&(width as u16).to_be_bytes());
    header[10..12].copy_from_slice(&(height as u16).to_be_bytes());
    header[12..14].copy_from_slice(&1_u16.to_be_bytes());
    header[14..16].copy_from_slice(&(width as u16).to_be_bytes());
    header[16..18].copy_from_slice(&(height as u16).to_be_bytes());
    header[18..20].copy_from_slice(&1_u16.to_be_bytes());
    header[20] = 1;
    header[21] = 32;
    header[28..32].copy_from_slice(&(IMG_XEN_HEADER_SIZE as u32).to_be_bytes());
    header[32..36].copy_from_slice(&payload_length.to_be_bytes());

    let mut img_xen = Vec::with_capacity(IMG_XEN_HEADER_SIZE + png.len());
    img_xen.extend_from_slice(&header);
    img_xen.extend_from_slice(png);
    Ok(img_xen)
}

fn category_logo_lines(category_number: usize, category_name: &str) -> [String; 3] {
    let prefix = format!("{category_number:02} ");
    let label_and_value = category_name.strip_prefix(&prefix).unwrap_or(category_name);
    let (label, value) = label_and_value
        .split_once(": ")
        .unwrap_or(("Category", label_and_value));

    [
        format!("{category_number:02}"),
        label.to_string(),
        value.to_string(),
    ]
}

fn render_category_logo(lines: &[String; 3], font: &FontArc) -> RgbaImage {
    let mut image =
        RgbaImage::from_pixel(CATEGORY_LOGO_SIZE, CATEGORY_LOGO_SIZE, Rgba([0, 0, 0, 0]));
    let line_layout = lines
        .iter()
        .map(|line| {
            let scale = category_logo_scale(line, font);
            let (_, height) = text_size(scale, font, line);
            (scale, height.max(1))
        })
        .collect::<Vec<_>>();
    let total_height = line_layout.iter().map(|(_, height)| height).sum::<u32>()
        + CATEGORY_LOGO_LINE_GAP * (lines.len() as u32 - 1);
    let mut y = ((CATEGORY_LOGO_SIZE.saturating_sub(total_height)) / 2) as i32;

    for (line, (scale, height)) in lines.iter().zip(line_layout) {
        let (width, _) = text_size(scale, font, line);
        let x = ((CATEGORY_LOGO_SIZE.saturating_sub(width)) / 2) as i32;
        draw_outlined_category_logo_text(&mut image, line, x, y, scale, font);
        y += height as i32 + CATEGORY_LOGO_LINE_GAP as i32;
    }

    image
}

fn category_logo_scale(text: &str, font: &FontArc) -> PxScale {
    for size in (8..=128).rev() {
        let scale = PxScale::from(size as f32);
        let (width, height) = text_size(scale, font, text);
        if width + (CATEGORY_LOGO_STROKE_WIDTH as u32 * 2) <= CATEGORY_LOGO_MAX_LINE_WIDTH
            && height + (CATEGORY_LOGO_STROKE_WIDTH as u32 * 2) <= CATEGORY_LOGO_MAX_LINE_HEIGHT
        {
            return scale;
        }
    }

    PxScale::from(8.0)
}

fn draw_outlined_category_logo_text(
    image: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    scale: PxScale,
    font: &FontArc,
) {
    for offset_y in -CATEGORY_LOGO_STROKE_WIDTH..=CATEGORY_LOGO_STROKE_WIDTH {
        for offset_x in -CATEGORY_LOGO_STROKE_WIDTH..=CATEGORY_LOGO_STROKE_WIDTH {
            if offset_x * offset_x + offset_y * offset_y
                <= CATEGORY_LOGO_STROKE_WIDTH * CATEGORY_LOGO_STROKE_WIDTH
            {
                draw_text_mut(
                    image,
                    Rgba([0, 0, 0, 255]),
                    x + offset_x,
                    y + offset_y,
                    scale,
                    font,
                    text,
                );
            }
        }
    }
    draw_text_mut(image, Rgba([255, 255, 255, 255]), x, y, scale, font, text);
}

fn scan_song_ini_files_paths_with_progress<F>(
    settings: &ScanSettings,
    store: &SongIniStore,
    emit_progress: F,
) -> Result<SongIniScanResult, String>
where
    F: FnMut(SongScanProgress),
{
    let mods_dir = &settings.mods_dir;
    let mut progress_emitter = SongScanProgressEmitter::new(emit_progress);
    progress_emitter.emit(song_scan_progress("findingSongs", 0, 0, ""), true);
    let song_ini_paths = find_scanned_song_ini_files(mods_dir)?;
    let mut result = SongIniScanResult {
        songs_found: song_ini_paths.len(),
        ..SongIniScanResult::default()
    };
    let mut parsed_songs = Vec::new();

    for (index, song_ini_path) in song_ini_paths.iter().enumerate() {
        let relative_path = mods_relative_path(mods_dir, &song_ini_path)?;
        progress_emitter.emit(
            song_scan_progress(
                "readingSongs",
                index + 1,
                song_ini_paths.len(),
                &relative_path,
            ),
            false,
        );
        let contents = match fs::read_to_string(&song_ini_path) {
            Ok(contents) => contents,
            Err(err) => {
                result
                    .errors
                    .push(format!("Failed to read {}: {err}", song_ini_path.display()));
                continue;
            }
        };

        let normalized_contents = normalize_song_ini_key_case(&contents);
        match parse_song_ini(&relative_path, &normalized_contents) {
            Ok(parsed_song) => {
                if normalized_contents != contents {
                    if let Err(err) = write_song_ini_file(
                        &song_ini_path,
                        &normalized_contents,
                        settings.keep_original_song_ini,
                    ) {
                        result.errors.push(format!(
                            "Failed to save corrected key casing in {}: {err}",
                            song_ini_path.display()
                        ));
                    }
                }
                parsed_songs.push(parsed_song);
            }
            Err(error) => result.faulty_files.push(FaultySongIniFile {
                relative_path,
                contents,
                error,
            }),
        }
    }

    result.songs_parsed = parsed_songs.len();
    result.songs = scanned_songs(mods_dir, &parsed_songs);
    result.duplicate_checksum_groups = duplicate_checksum_groups(&parsed_songs);
    result.song_ini_folder_conflicts = song_ini_folder_conflicts(mods_dir)?;
    result.content_file_issues = song_content_issues_with_progress(
        mods_dir,
        &parsed_songs,
        |current, total, relative_path| {
            progress_emitter.emit(
                song_scan_progress("checkingContent", current, total, relative_path),
                false,
            );
        },
    )?;
    progress_emitter.emit(song_scan_progress("finishing", 0, 0, ""), true);
    replace_song_ini_store(store, parsed_songs)?;
    Ok(result)
}

struct SongScanProgressEmitter<F>
where
    F: FnMut(SongScanProgress),
{
    emit_progress: F,
    last_emit: Option<Instant>,
    last_phase: Option<String>,
}

impl<F> SongScanProgressEmitter<F>
where
    F: FnMut(SongScanProgress),
{
    fn new(emit_progress: F) -> Self {
        Self {
            emit_progress,
            last_emit: None,
            last_phase: None,
        }
    }

    fn emit(&mut self, progress: SongScanProgress, force: bool) {
        let phase_changed = self.last_phase.as_deref() != Some(progress.phase.as_str());
        if !force
            && !phase_changed
            && self
                .last_emit
                .map(|last_emit| last_emit.elapsed() < Duration::from_secs(1))
                .unwrap_or(false)
        {
            return;
        }

        self.last_phase = Some(progress.phase.clone());
        (self.emit_progress)(progress);
        self.last_emit = Some(Instant::now());
    }
}

fn scan_game_icon_categories_paths(
    settings: &ScanSettings,
) -> Result<GameIconCategoryScanResult, String> {
    let mut errors = Vec::new();
    let mut gamelogo_paths = Vec::new();
    collect_gamelogo_files(
        &settings.mods_dir,
        &settings.mods_dir,
        &mut gamelogo_paths,
        &mut errors,
    );
    gamelogo_paths.sort();
    Ok(game_icon_category_scan_result(
        settings,
        gamelogo_paths,
        errors,
    ))
}

fn fix_game_icon_categories_paths(
    settings: &ScanSettings,
) -> Result<GameIconCategoryScanResult, String> {
    let initial = scan_game_icon_categories_paths(settings)?;
    let mut errors = initial.errors;

    for group in initial
        .groups
        .iter()
        .filter(|group| group.gamelogos.len() > 1)
    {
        let group_dir = PathBuf::from(&group.folder_absolute_path);
        let category_ini = find_category_ini(&group_dir).ok().flatten();
        let category_logo = category_ini
            .as_ref()
            .and_then(|path| {
                category_ini_logo(path, &mods_relative_path(&settings.mods_dir, path).ok()?).ok()
            })
            .flatten();

        for gamelogo in &group.gamelogos {
            let source = settings.mods_dir.join(Path::new(&gamelogo.relative_path));
            let target_dir = group_dir.join(format!("Category_{}", gamelogo.stem));
            let target = target_dir.join(&gamelogo.file_name);
            let should_move_category = category_ini.is_some()
                && category_logo
                    .as_deref()
                    .is_some_and(|logo| logo.eq_ignore_ascii_case(&gamelogo.stem));
            let target_category = target_dir.join("category.ini");

            if target.exists() {
                errors.push(format!(
                    "Skipped {} because {} already exists.",
                    source.display(),
                    target.display()
                ));
                continue;
            }
            if should_move_category && target_category.exists() {
                errors.push(format!(
                    "Skipped {} because {} already exists.",
                    source.display(),
                    target_category.display()
                ));
                continue;
            }

            if let Err(err) = fs::create_dir_all(&target_dir) {
                errors.push(format!(
                    "Failed to create folder {}: {err}",
                    target_dir.display()
                ));
                continue;
            }
            if let Err(err) = fs::rename(&source, &target) {
                errors.push(format!(
                    "Failed to move {} to {}: {err}",
                    source.display(),
                    target.display()
                ));
                continue;
            }
            if should_move_category {
                if let Some(category_ini) = &category_ini {
                    if let Err(err) = fs::rename(category_ini, &target_category) {
                        errors.push(format!(
                            "Failed to move {} to {}: {err}",
                            category_ini.display(),
                            target_category.display()
                        ));
                    }
                }
            }
        }

        if let Some(category_ini) = &category_ini {
            let category_matches_group = category_logo.as_deref().is_some_and(|logo| {
                group
                    .gamelogos
                    .iter()
                    .any(|gamelogo| logo.eq_ignore_ascii_case(&gamelogo.stem))
            });

            if category_ini.exists() && !category_matches_group {
                let backup_path = unique_sibling_path(
                    &category_ini.with_file_name("category.original.faulty.ini"),
                );
                if let Err(err) = fs::rename(category_ini, &backup_path) {
                    errors.push(format!(
                        "Failed to rename {} to {}: {err}",
                        category_ini.display(),
                        backup_path.display()
                    ));
                }
            }
        }
    }

    let mut gamelogo_paths = Vec::new();
    collect_gamelogo_files(
        &settings.mods_dir,
        &settings.mods_dir,
        &mut gamelogo_paths,
        &mut errors,
    );
    gamelogo_paths.sort();
    let grouped_paths = game_icon_paths_by_folder(&gamelogo_paths);

    for (folder, paths) in grouped_paths {
        for gamelogo_path in paths {
            let Some(stem) = gamelogo_stem(&gamelogo_path) else {
                continue;
            };
            if let Err(err) = ensure_gamelogo_category_ini(settings, &folder, &stem) {
                errors.push(err);
            }
        }
    }

    let mut refreshed_paths = Vec::new();
    collect_gamelogo_files(
        &settings.mods_dir,
        &settings.mods_dir,
        &mut refreshed_paths,
        &mut errors,
    );
    refreshed_paths.sort();
    Ok(game_icon_category_scan_result(
        settings,
        refreshed_paths,
        errors,
    ))
}

fn preview_game_icon_song_fixes_paths(
    settings: &ScanSettings,
    store: &SongIniStore,
) -> Result<GameIconSongFixPreview, String> {
    let parsed_songs = stored_parsed_songs(store)?;
    if parsed_songs.is_empty() {
        return Err("Scan MODS folder before fixing song GameIcons.".to_string());
    }

    Ok(game_icon_song_fix_preview(settings, &parsed_songs))
}

fn apply_game_icon_song_fixes_paths(
    settings: &ScanSettings,
    fixes: Vec<GameIconSongFixInput>,
    store: &SongIniStore,
) -> Result<GameIconSongFixApplyResult, String> {
    if fixes.is_empty() {
        return Err("No GameIcon fixes were provided.".to_string());
    }

    let mut parsed_songs = stored_parsed_songs(store)?;
    if parsed_songs.is_empty() {
        return Err("Scan MODS folder before fixing song GameIcons.".to_string());
    }

    let valid_game_icons = game_icon_valid_stems(settings);
    let valid_game_icon_keys = lower_value_set(&valid_game_icons);
    let mut seen_paths = HashSet::new();

    for fix in &fixes {
        let new_game_icon = fix.new_game_icon.trim();
        if !new_game_icon.is_empty()
            && !valid_game_icon_keys.contains(&new_game_icon.to_ascii_lowercase())
        {
            return Err(format!(
                "{} is not a known official or custom GameIcon.",
                fix.new_game_icon
            ));
        }
        if !seen_paths.insert(fix.relative_path.clone()) {
            return Err(format!(
                "{} was provided more than once.",
                fix.relative_path
            ));
        }
        if !parsed_songs
            .iter()
            .any(|song| song.relative_path == fix.relative_path)
        {
            return Err(format!(
                "{} is not in the current scanned songs list.",
                fix.relative_path
            ));
        }
    }

    for fix in fixes {
        let parsed_song =
            update_song_game_icon_path(settings, &fix.relative_path, &fix.new_game_icon)?;
        if let Some(existing_song) = parsed_songs
            .iter_mut()
            .find(|song| song.relative_path == parsed_song.relative_path)
        {
            *existing_song = parsed_song;
        }
    }

    parsed_songs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    replace_song_ini_store(store, parsed_songs.clone())?;
    Ok(GameIconSongFixApplyResult {
        applied: seen_paths.len(),
        songs_parsed: parsed_songs.len(),
        songs: scanned_songs(&settings.mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        song_ini_folder_conflicts: song_ini_folder_conflicts(&settings.mods_dir)?,
        content_file_issues: song_content_issues(&settings.mods_dir, &parsed_songs)?,
    })
}

fn game_icon_song_fix_preview(
    settings: &ScanSettings,
    parsed_songs: &[ParsedSongIni],
) -> GameIconSongFixPreview {
    let scanned_songs = scanned_songs(&settings.mods_dir, parsed_songs);
    let valid_game_icons = game_icon_valid_stems(settings);
    let valid_game_icon_keys = lower_value_set(&valid_game_icons);
    let majority_icons = parent_folder_majority_game_icons(&scanned_songs, &valid_game_icon_keys);
    let custom_locations = custom_game_icon_locations(settings);
    let mut rows = scanned_songs
        .into_iter()
        .filter(|song| !valid_game_icon_keys.contains(&song.game_icon.to_ascii_lowercase()))
        .map(|song| {
            let parent_relative_path = song_parent_folder(&song.relative_path).unwrap_or_default();
            let new_game_icon =
                guess_song_game_icon(&song.relative_path, &majority_icons, &custom_locations)
                    .unwrap_or_default();

            GameIconSongFixRow {
                new_game_icon,
                relative_path: song.relative_path,
                parent_relative_path,
                artist: song.artist,
                title: song.title,
                invalid_game_icon: song.game_icon,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    GameIconSongFixPreview {
        rows,
        valid_game_icons,
    }
}

fn game_icon_valid_stems(settings: &ScanSettings) -> Vec<String> {
    let mut stems = official_game_icon_stems(settings);
    stems.extend(
        custom_game_icon_locations(settings)
            .into_iter()
            .map(|location| location.stem),
    );
    stems.sort_by_key(|value| value.to_ascii_lowercase());
    stems.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    stems
}

fn lower_value_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn custom_game_icon_locations(settings: &ScanSettings) -> Vec<CustomGameIconLocation> {
    let mut errors = Vec::new();
    let mut gamelogo_paths = Vec::new();
    collect_gamelogo_files(
        &settings.mods_dir,
        &settings.mods_dir,
        &mut gamelogo_paths,
        &mut errors,
    );
    gamelogo_paths.sort();

    game_icon_paths_by_folder(&gamelogo_paths)
        .into_iter()
        .flat_map(|(folder, paths)| {
            game_icon_category_group(settings, &folder, &paths)
                .gamelogos
                .into_iter()
                .map(move |gamelogo| CustomGameIconLocation {
                    stem: gamelogo.stem,
                    folder_relative_path: mods_relative_path(&settings.mods_dir, &folder)
                        .unwrap_or_default(),
                    has_matching_category_ini: gamelogo.has_matching_category_ini,
                })
        })
        .collect()
}

fn parent_folder_majority_game_icons(
    songs: &[ScannedSong],
    valid_game_icon_keys: &HashSet<String>,
) -> HashMap<String, String> {
    let mut counts: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    for song in songs {
        if !valid_game_icon_keys.contains(&song.game_icon.to_ascii_lowercase()) {
            continue;
        }
        let Some(parent_folder) = song_parent_folder(&song.relative_path) else {
            continue;
        };
        let icon_counts = counts.entry(parent_folder).or_default();

        if let Some((_, count)) = icon_counts
            .iter_mut()
            .find(|(game_icon, _)| game_icon.eq_ignore_ascii_case(&song.game_icon))
        {
            *count += 1;
        } else {
            icon_counts.push((song.game_icon.clone(), 1));
        }
    }

    counts
        .into_iter()
        .filter_map(|(folder, icon_counts)| {
            let mut selected: Option<(String, usize)> = None;

            for (game_icon, count) in icon_counts {
                if selected
                    .as_ref()
                    .is_none_or(|(_, selected_count)| count > *selected_count)
                {
                    selected = Some((game_icon, count));
                }
            }

            Some((folder, selected?.0))
        })
        .collect()
}

fn song_parent_folder(relative_path: &str) -> Option<String> {
    let song_folder = parent_relative_path(relative_path).unwrap_or_default();
    Some(parent_relative_path(&song_folder).unwrap_or_default())
}

fn guess_song_game_icon(
    relative_path: &str,
    majority_icons: &HashMap<String, String>,
    custom_locations: &[CustomGameIconLocation],
) -> Option<String> {
    if let Some(parent_folder) = song_parent_folder(relative_path) {
        if let Some(game_icon) = majority_icons.get(&parent_folder) {
            return Some(game_icon.clone());
        }
    }

    let folder = song_parent_folder(relative_path)?;
    custom_locations
        .iter()
        .find(|location| {
            location.has_matching_category_ini && location.folder_relative_path == folder
        })
        .or_else(|| {
            custom_locations.iter().find(|location| {
                location.has_matching_category_ini
                    && is_direct_child_relative_path(&folder, &location.folder_relative_path)
            })
        })
        .map(|location| location.stem.clone())
}

fn parent_relative_path(relative_path: &str) -> Option<String> {
    relative_path
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
}

fn is_direct_child_relative_path(parent: &str, child: &str) -> bool {
    if parent.is_empty() {
        return !child.is_empty() && !child.contains('/');
    }

    let Some(remainder) = child.strip_prefix(parent) else {
        return false;
    };
    let Some(remainder) = remainder.strip_prefix('/') else {
        return false;
    };

    !remainder.is_empty() && !remainder.contains('/')
}

fn update_song_game_icon_path(
    settings: &ScanSettings,
    relative_path: &str,
    game_icon: &str,
) -> Result<ParsedSongIni, String> {
    let path = checked_mods_relative_path(&settings.mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let normalized_contents = normalize_song_ini_key_case(&contents);
    let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;
    let updated_contents = if game_icon.trim().is_empty() {
        remove_song_info_value(&normalized_contents, "GameIcon")
    } else {
        let metadata = ScannedSongMetadataInput {
            artist: song_info_value(&parsed_song, "Artist"),
            title: song_info_value(&parsed_song, "Title"),
            year: song_info_value(&parsed_song, "Year"),
            genre: song_info_value(&parsed_song, "Genre"),
            game_icon: game_icon.trim().to_string(),
        };
        update_song_ini_metadata_contents(&normalized_contents, &metadata)
    };
    let parsed_song = parse_song_ini(relative_path, &updated_contents)?;

    write_song_ini_file(&path, &updated_contents, settings.keep_original_song_ini)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    Ok(parsed_song)
}

fn game_icon_category_scan_result(
    settings: &ScanSettings,
    gamelogo_paths: Vec<PathBuf>,
    errors: Vec<String>,
) -> GameIconCategoryScanResult {
    let grouped_paths = game_icon_paths_by_folder(&gamelogo_paths);
    let mut groups = grouped_paths
        .into_iter()
        .map(|(folder, paths)| game_icon_category_group(settings, &folder, &paths))
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.folder_relative_path.cmp(&right.folder_relative_path));

    let needs_fix = groups.iter().any(|group| {
        group.has_multiple_gamelogos
            || group
                .gamelogos
                .iter()
                .any(|gamelogo| !gamelogo.has_matching_category_ini)
    });
    let mut custom_game_icons = groups
        .iter()
        .flat_map(|group| group.gamelogos.iter().map(|gamelogo| gamelogo.stem.clone()))
        .collect::<Vec<_>>();
    custom_game_icons.sort_by_key(|value| value.to_ascii_lowercase());
    custom_game_icons.dedup_by(|left, right| left.eq_ignore_ascii_case(right));

    GameIconCategoryScanResult {
        groups,
        needs_fix,
        official_game_icons: official_game_icon_stems(settings),
        custom_game_icons,
        errors,
    }
}

fn game_icon_category_group(
    settings: &ScanSettings,
    folder: &Path,
    paths: &[PathBuf],
) -> GameIconCategoryGroup {
    let category_ini = find_category_ini(folder).ok().flatten();
    let category_logo = category_ini
        .as_ref()
        .and_then(|path| {
            category_ini_logo(path, &mods_relative_path(&settings.mods_dir, path).ok()?).ok()
        })
        .flatten();
    let mut gamelogos = paths
        .iter()
        .filter_map(|path| {
            let file_name = path.file_name()?.to_string_lossy().into_owned();
            let stem = gamelogo_stem(path)?;
            let has_matching_category_ini = category_logo
                .as_deref()
                .is_some_and(|logo| logo.eq_ignore_ascii_case(&stem));

            Some(GameIconFile {
                relative_path: mods_relative_path(&settings.mods_dir, path).ok()?,
                file_name,
                stem,
                has_matching_category_ini,
            })
        })
        .collect::<Vec<_>>();
    gamelogos.sort_by(|left, right| left.file_name.cmp(&right.file_name));

    GameIconCategoryGroup {
        folder_relative_path: mods_relative_path(&settings.mods_dir, folder).unwrap_or_default(),
        folder_absolute_path: folder.display().to_string(),
        has_multiple_gamelogos: gamelogos.len() > 1,
        gamelogos,
    }
}

fn game_icon_paths_by_folder(paths: &[PathBuf]) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let mut paths_by_folder: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        if let Some(folder) = path.parent() {
            paths_by_folder
                .entry(folder.to_path_buf())
                .or_default()
                .push(path.clone());
        }
    }

    let mut grouped_paths = paths_by_folder.into_iter().collect::<Vec<_>>();
    grouped_paths.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (_, paths) in &mut grouped_paths {
        paths.sort();
    }
    grouped_paths
}

fn ensure_gamelogo_category_ini(
    settings: &ScanSettings,
    folder: &Path,
    gamelogo_stem: &str,
) -> Result<(), String> {
    let category_ini = find_category_ini(folder).map_err(|err| {
        format!(
            "Failed to inspect category.ini in {}: {err}",
            folder.display()
        )
    })?;
    let category_matches = category_ini
        .as_ref()
        .and_then(|path| {
            category_ini_logo(
                path,
                &mods_relative_path(&settings.mods_dir, path)
                    .unwrap_or_else(|_| "category.ini".to_string()),
            )
            .ok()
        })
        .flatten()
        .is_some_and(|logo| logo.eq_ignore_ascii_case(gamelogo_stem));

    if category_matches {
        return Ok(());
    }

    if let Some(category_ini) = category_ini {
        let backup_path =
            unique_sibling_path(&category_ini.with_file_name("category.original.faulty.ini"));
        fs::rename(&category_ini, &backup_path).map_err(|err| {
            format!(
                "Failed to rename {} to {}: {err}",
                category_ini.display(),
                backup_path.display()
            )
        })?;
    }

    let category_path = folder.join("category.ini");
    fs::write(
        &category_path,
        game_icon_category_ini_contents(gamelogo_stem),
    )
    .map_err(|err| format!("Failed to write {}: {err}", category_path.display()))
}

fn category_ini_logo(path: &Path, relative_path: &str) -> Result<Option<String>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let parse_contents = contents.trim_start_matches('\u{feff}');
    validate_unique_ini_keys(relative_path, parse_contents)?;
    let ini = Ini::load_from_str(parse_contents)
        .map_err(|err| format!("Failed to parse {relative_path}: {err}"))?;

    Ok(ini_string(&ini, "CategoryInfo", "Logo"))
}

fn find_category_ini(folder: &Path) -> io::Result<Option<PathBuf>> {
    let mut matches = Vec::new();

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.eq_ignore_ascii_case("category.ini") && entry.file_type()?.is_file() {
            matches.push(entry.path());
        }
    }

    matches.sort();
    Ok(matches.into_iter().next())
}

fn collect_gamelogo_files(
    mods_dir: &Path,
    dir: &Path,
    paths: &mut Vec<PathBuf>,
    errors: &mut Vec<String>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(format!("Failed to read folder {}: {err}", dir.display()));
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                errors.push(format!("Failed to read entry in {}: {err}", dir.display()));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                errors.push(format!("Failed to inspect {}: {err}", path.display()));
                continue;
            }
        };

        if file_type.is_dir() {
            if dir == mods_dir && is_ini_tool_categories_dir(&path) {
                continue;
            }
            collect_gamelogo_files(mods_dir, &path, paths, errors);
        } else if file_type.is_file() && is_gamelogo_img_xen(&path) {
            paths.push(path);
        }
    }
}

fn is_ini_tool_categories_dir(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        name.to_string_lossy()
            .eq_ignore_ascii_case("IniToolCategories")
    })
}

fn official_game_icon_stems(settings: &ScanSettings) -> Vec<String> {
    let Some(dir) = &settings.official_gamelogos_dir else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut stems = Vec::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() && is_gamelogo_img_xen(&entry.path()) {
            if let Some(stem) = gamelogo_stem(&entry.path()) {
                stems.push(stem);
            }
        }
    }

    stems.sort_by_key(|value| value.to_ascii_lowercase());
    stems.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    stems
}

fn is_gamelogo_img_xen(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let lower_name = name.to_ascii_lowercase();
            lower_name.starts_with("gamelogo_") && lower_name.ends_with(".img.xen")
        })
}

fn gamelogo_stem(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    is_gamelogo_img_xen(path).then(|| file_name[..file_name.len() - ".img.xen".len()].to_string())
}

fn game_icon_category_ini_contents(gamelogo_stem: &str) -> String {
    let label = gamelogo_stem
        .strip_prefix("gamelogo_")
        .unwrap_or(gamelogo_stem);

    format!(
        "[ModInfo]\nName=GameIcon {label} Category\nDescription=Used for GameIcon, no songs linked\nAuthor=GhwtDeIniTool\nVersion=1.0\n\n[CategoryInfo]\nName=GameIcon {label}\nChecksum={label}\nLogo={gamelogo_stem}\n"
    )
}

fn unique_sibling_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("category.original.faulty");
    let extension = path.extension().and_then(|extension| extension.to_str());

    for index in 1.. {
        let file_name = match extension {
            Some(extension) => format!("{stem}.{index}.{extension}"),
            None => format!("{stem}.{index}"),
        };
        let candidate = parent.join(file_name);

        if !candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

#[cfg(test)]
fn scan_song_ini_files_paths(
    settings: &ScanSettings,
    store: &SongIniStore,
) -> Result<SongIniScanResult, String> {
    scan_song_ini_files_paths_with_progress(settings, store, |_| {})
}

fn validate_song_ini_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    contents: &str,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    let normalized_contents = normalize_song_ini_key_case(contents);
    let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;

    write_repaired_song_ini_file(&path, &normalized_contents, settings.keep_original_song_ini)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    let parsed_songs = upsert_song_ini_store(store, parsed_song)?;
    let songs_parsed = parsed_songs.len();
    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents: normalized_contents,
        songs_parsed,
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        song_ini_folder_conflicts: song_ini_folder_conflicts(mods_dir)?,
        content_file_issues: song_content_issues(mods_dir, &parsed_songs)?,
    })
}

fn undo_song_ini_repair_path(
    settings: &ScanSettings,
    relative_path: &str,
    contents: &str,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    fs::write(&path, contents)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    let parsed_songs = remove_song_ini_from_store_with_songs(store, relative_path)?;
    let songs_parsed = parsed_songs.len();
    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents: contents.to_string(),
        songs_parsed,
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        song_ini_folder_conflicts: song_ini_folder_conflicts(mods_dir)?,
        content_file_issues: song_content_issues(mods_dir, &parsed_songs)?,
    })
}

fn verify_content_issue_songs_paths(
    settings: &ScanSettings,
    relative_paths: &[String],
    store: &SongIniStore,
) -> Result<SongContentVerificationResult, String> {
    let mods_dir = &settings.mods_dir;
    let mut paths_to_remove = HashSet::new();
    let mut parsed_song_updates = Vec::new();
    let mut seen_paths = HashSet::new();

    for relative_path in relative_paths {
        if !seen_paths.insert(relative_path.as_str()) {
            continue;
        }

        let unresolved_path = checked_mods_relative_path_allow_missing(mods_dir, relative_path)?;

        if !is_scanned_song_ini(&unresolved_path) {
            return Err(format!(
                "{relative_path} is not a song.ini or song.excluded.ini file."
            ));
        }

        if !unresolved_path.is_file() {
            paths_to_remove.insert(relative_path.as_str());
            continue;
        }

        let path = checked_mods_relative_path(mods_dir, relative_path)?;
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let normalized_contents = normalize_song_ini_key_case(&contents);
        let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;

        parsed_song_updates.push(parsed_song);
    }

    let parsed_songs = {
        let mut songs = store
            .0
            .lock()
            .map_err(|_| "Failed to lock song.ini store.".to_string())?;

        songs.retain(|song| !paths_to_remove.contains(song.relative_path.as_str()));

        for parsed_song in parsed_song_updates {
            if let Some(existing_song) = songs
                .iter_mut()
                .find(|song| song.relative_path == parsed_song.relative_path)
            {
                *existing_song = parsed_song;
            } else {
                songs.push(parsed_song);
            }
        }

        songs.clone()
    };

    Ok(SongContentVerificationResult {
        songs_parsed: parsed_songs.len(),
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        song_ini_folder_conflicts: song_ini_folder_conflicts(mods_dir)?,
        content_file_issues: song_content_issues(mods_dir, &parsed_songs)?,
    })
}

fn disable_song_ini_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniDisableResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    if let Some(disabled_path) = find_sibling_disabled_song_ini(&path)? {
        return Err(format!(
            "Cannot disable {relative_path} because {} already exists.",
            mods_relative_path(mods_dir, &disabled_path)?
        ));
    }

    if settings.keep_original_song_ini {
        backup_original_song_ini(&path)
            .map_err(|err| format!("Failed to back up {}: {err}", path.display()))?;
    }

    let disabled_path = path.with_file_name("song.disabled.ini");
    fs::rename(&path, &disabled_path).map_err(|err| {
        format!(
            "Failed to disable {} as {}: {err}",
            path.display(),
            disabled_path.display()
        )
    })?;

    let songs_parsed = remove_song_ini_from_store(store, relative_path)?;

    Ok(SongIniDisableResult {
        relative_path: relative_path.to_string(),
        disabled_path: mods_relative_path(mods_dir, &disabled_path)?,
        is_included: is_song_ini(&path),
        songs_parsed,
    })
}

fn enable_song_ini_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniEnableResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path_allow_missing(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    if path.exists() {
        return Err(format!("{relative_path} is already enabled."));
    }

    let opposite_path = song_ini_path_for_inclusion(&path, !is_song_ini(&path));

    if opposite_path.exists() {
        return Err(format!(
            "Cannot enable {relative_path} because {} already exists.",
            mods_relative_path(mods_dir, &opposite_path)?
        ));
    }

    let disabled_path = path.with_file_name("song.disabled.ini");

    if !disabled_path.is_file() {
        return Err(format!(
            "{} has no disabled song.ini file to enable.",
            relative_path
        ));
    }

    let disabled_contents = fs::read_to_string(&disabled_path)
        .map_err(|err| format!("Failed to read {}: {err}", disabled_path.display()))?;
    let parsed_song = parse_song_ini(
        relative_path,
        &normalize_song_ini_key_case(&disabled_contents),
    )
    .ok();

    fs::rename(&disabled_path, &path).map_err(|err| {
        format!(
            "Failed to enable {} as {}: {err}",
            disabled_path.display(),
            path.display()
        )
    })?;

    let songs_parsed = match parsed_song {
        Some(parsed_song) => upsert_song_ini_store(store, parsed_song)?.len(),
        None => store_song_count(store)?,
    };

    Ok(SongIniEnableResult {
        relative_path: relative_path.to_string(),
        enabled_path: mods_relative_path(mods_dir, &path)?,
        is_included: is_song_ini(&path),
        songs_parsed,
    })
}

fn delete_song_ini_conflict_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniDeleteResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) && !is_disabled_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini, song.excluded.ini, or song.disabled.ini file."
        ));
    }

    fs::remove_file(&path).map_err(|err| format!("Failed to delete {}: {err}", path.display()))?;

    let songs_parsed = if is_scanned_song_ini(&path) {
        remove_song_ini_from_store(store, relative_path)?
    } else {
        store_song_count(store)?
    };

    Ok(SongIniDeleteResult {
        relative_path: relative_path.to_string(),
        songs_parsed,
    })
}

fn set_scanned_song_included_path(
    settings: &ScanSettings,
    relative_path: &str,
    is_included: bool,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    let target_path = song_ini_path_for_inclusion(&path, is_included);
    let target_relative_path = mods_relative_path(mods_dir, &target_path)?;
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let normalized_contents = normalize_song_ini_key_case(&contents);
    let parsed_song = parse_song_ini(&target_relative_path, &normalized_contents)?;

    move_scanned_song_ini_to_target(&path, &target_path)?;

    if path != target_path {
        remove_song_ini_from_store_with_songs(store, relative_path)?;
    }

    refreshed_song_ini_result(
        settings,
        &target_relative_path,
        normalized_contents,
        parsed_song,
        store,
    )
}

fn update_scanned_song_metadata_path(
    settings: &ScanSettings,
    relative_path: &str,
    metadata: ScannedSongMetadataInput,
    is_included: bool,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    if metadata.artist.trim().is_empty() {
        return Err("Artist is required.".to_string());
    }
    if metadata.title.trim().is_empty() {
        return Err("Title is required.".to_string());
    }

    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    let target_path = song_ini_path_for_inclusion(&path, is_included);
    let target_relative_path = mods_relative_path(mods_dir, &target_path)?;

    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let normalized_contents = normalize_song_ini_key_case(&contents);
    parse_song_ini(&target_relative_path, &normalized_contents)?;
    let updated_contents = update_song_ini_metadata_contents(&normalized_contents, &metadata);
    let parsed_song = parse_song_ini(&target_relative_path, &updated_contents)?;

    write_scanned_song_ini_file(
        &path,
        &target_path,
        &updated_contents,
        settings.keep_original_song_ini,
    )?;

    if path != target_path {
        remove_song_ini_from_store_with_songs(store, relative_path)?;
    }

    refreshed_song_ini_result(
        settings,
        &target_relative_path,
        updated_contents,
        parsed_song,
        store,
    )
}

fn restore_original_song_ini_path(
    settings: &ScanSettings,
    relative_path: &str,
    is_included: bool,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_scanned_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.excluded.ini file."
        ));
    }

    let target_path = song_ini_path_for_inclusion(&path, is_included);
    let target_relative_path = mods_relative_path(mods_dir, &target_path)?;

    let backup_path = original_song_ini_path(&path);

    if !backup_path.is_file() {
        return Err(format!(
            "{} has no original song.ini backup to restore.",
            relative_path
        ));
    }

    let backup_contents = fs::read_to_string(&backup_path)
        .map_err(|err| format!("Failed to read {}: {err}", backup_path.display()))?;
    let parsed_song = parse_song_ini(
        &target_relative_path,
        &normalize_song_ini_key_case(&backup_contents),
    )?;

    move_scanned_song_ini_to_target(&path, &target_path)?;
    fs::copy(&backup_path, &target_path).map_err(|err| {
        format!(
            "Failed to restore {} from {}: {err}",
            target_path.display(),
            backup_path.display()
        )
    })?;
    fs::remove_file(&backup_path)
        .map_err(|err| format!("Failed to remove {}: {err}", backup_path.display()))?;

    if path != target_path {
        remove_song_ini_from_store_with_songs(store, relative_path)?;
    }

    refreshed_song_ini_result(
        settings,
        &target_relative_path,
        backup_contents,
        parsed_song,
        store,
    )
}

fn restore_all_ini_paths(
    settings: &ScanSettings,
    action: RestoreIniAction,
    store: &SongIniStore,
) -> Result<RestoreIniResult, String> {
    let mods_dir = &settings.mods_dir;

    if action == RestoreIniAction::DeleteInstrumentSidecars {
        let instrument_sidecar_paths = find_instrument_sidecar_files(mods_dir)?;
        let mut result = RestoreIniResult::default();

        delete_instrument_sidecars(&instrument_sidecar_paths, &mut result);
        replace_song_ini_store(store, Vec::new())?;
        return Ok(result);
    }

    let excluded_song_ini_paths = find_excluded_song_ini_files(mods_dir)?;
    let mut result = RestoreIniResult::default();
    let mut restored_paths = HashSet::new();

    restore_all_from_excluded_song_ini_files(
        &excluded_song_ini_paths,
        mods_dir,
        &mut result,
        &mut restored_paths,
    );

    let song_ini_paths = find_song_ini_files(mods_dir)?;
    restore_all_from_original_backups(&song_ini_paths, mods_dir, &mut result, &mut restored_paths);

    if action == RestoreIniAction::BeforeFormatIssueFix {
        restore_all_from_faulty_original_backups(
            &song_ini_paths,
            mods_dir,
            &mut result,
            &mut restored_paths,
        );
    }

    result.files_restored = restored_paths.len();
    replace_song_ini_store(store, Vec::new())?;
    Ok(result)
}

fn delete_instrument_sidecars(instrument_sidecar_paths: &[PathBuf], result: &mut RestoreIniResult) {
    for instrument_sidecar_path in instrument_sidecar_paths {
        match remove_existing_file(instrument_sidecar_path) {
            Ok(()) => result.files_deleted += 1,
            Err(err) => result.errors.push(err),
        }
    }
}

fn restore_all_from_excluded_song_ini_files(
    excluded_song_ini_paths: &[PathBuf],
    mods_dir: &Path,
    result: &mut RestoreIniResult,
    restored_paths: &mut HashSet<String>,
) {
    for excluded_song_ini_path in excluded_song_ini_paths {
        let song_ini_path = excluded_song_ini_path.with_file_name("song.ini");
        let relative_path = match mods_relative_path(mods_dir, &song_ini_path) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                result.errors.push(err);
                continue;
            }
        };

        if song_ini_path.exists() {
            result.errors.push(format!(
                "Failed to reactivate {} because {} already exists.",
                excluded_song_ini_path.display(),
                song_ini_path.display()
            ));
            continue;
        }

        if let Err(err) = fs::rename(excluded_song_ini_path, &song_ini_path) {
            result.errors.push(format!(
                "Failed to reactivate {} as {}: {err}",
                excluded_song_ini_path.display(),
                song_ini_path.display()
            ));
            continue;
        }

        restored_paths.insert(relative_path);
    }
}

fn restore_all_from_original_backups(
    song_ini_paths: &[PathBuf],
    mods_dir: &Path,
    result: &mut RestoreIniResult,
    restored_paths: &mut HashSet<String>,
) {
    for song_ini_path in song_ini_paths {
        let relative_path = match mods_relative_path(mods_dir, &song_ini_path) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                result.errors.push(err);
                continue;
            }
        };

        let backup_path = original_song_ini_path(&song_ini_path);

        match restore_song_ini_from_backup(&song_ini_path, &backup_path) {
            Ok(true) => {
                if let Err(err) = remove_existing_file(&backup_path) {
                    result.errors.push(err);
                } else {
                    restored_paths.insert(relative_path);
                }
            }
            Ok(false) => {}
            Err(err) => result.errors.push(err),
        }
    }
}

fn restore_all_from_faulty_original_backups(
    song_ini_paths: &[PathBuf],
    mods_dir: &Path,
    result: &mut RestoreIniResult,
    restored_paths: &mut HashSet<String>,
) {
    for song_ini_path in song_ini_paths {
        let relative_path = match mods_relative_path(mods_dir, &song_ini_path) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                result.errors.push(err);
                continue;
            }
        };

        let backup_path = faulty_original_song_ini_path(&song_ini_path);

        match restore_song_ini_from_backup(&song_ini_path, &backup_path) {
            Ok(true) => {
                if let Err(err) = remove_existing_file(&backup_path) {
                    result.errors.push(err);
                } else {
                    restored_paths.insert(relative_path);
                }
            }
            Ok(false) => {}
            Err(err) => result.errors.push(err),
        }
    }
}

fn restore_song_ini_from_backup(song_ini_path: &Path, backup_path: &Path) -> Result<bool, String> {
    if !backup_path.is_file() {
        return Ok(false);
    }

    fs::copy(&backup_path, song_ini_path).map_err(|err| {
        format!(
            "Failed to restore {} from {}: {err}",
            song_ini_path.display(),
            backup_path.display()
        )
    })?;

    Ok(true)
}

fn remove_existing_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to remove {}: {err}", path.display())),
    }
}

fn refreshed_song_ini_result(
    settings: &ScanSettings,
    relative_path: &str,
    contents: String,
    parsed_song: ParsedSongIni,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let parsed_songs = upsert_song_ini_store(store, parsed_song)?;
    let songs_parsed = parsed_songs.len();
    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents,
        songs_parsed,
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        song_ini_folder_conflicts: song_ini_folder_conflicts(mods_dir)?,
        content_file_issues: song_content_issues(mods_dir, &parsed_songs)?,
    })
}

fn write_song_ini_file(
    path: &Path,
    contents: &str,
    keep_original_song_ini: bool,
) -> io::Result<()> {
    if keep_original_song_ini {
        backup_original_song_ini(path)?;
    }

    fs::write(path, contents)
}

fn song_ini_path_for_inclusion(path: &Path, is_included: bool) -> PathBuf {
    path.with_file_name(if is_included {
        "song.ini"
    } else {
        "song.excluded.ini"
    })
}

fn move_scanned_song_ini_to_target(path: &Path, target_path: &Path) -> Result<(), String> {
    if path == target_path {
        return Ok(());
    }

    if target_path.exists() {
        return Err(format!(
            "Cannot move {} to {} because the destination already exists.",
            path.display(),
            target_path.display()
        ));
    }

    fs::rename(path, target_path).map_err(|err| {
        format!(
            "Failed to rename {} to {}: {err}",
            path.display(),
            target_path.display()
        )
    })
}

fn write_scanned_song_ini_file(
    path: &Path,
    target_path: &Path,
    contents: &str,
    keep_original_song_ini: bool,
) -> Result<(), String> {
    if path == target_path {
        return write_song_ini_file(path, contents, keep_original_song_ini)
            .map_err(|err| format!("Failed to write {}: {err}", path.display()));
    }

    if keep_original_song_ini {
        backup_original_song_ini(path)
            .map_err(|err| format!("Failed to back up {}: {err}", path.display()))?;
    }

    move_scanned_song_ini_to_target(path, target_path)?;
    fs::write(target_path, contents)
        .map_err(|err| format!("Failed to write {}: {err}", target_path.display()))
}

fn write_repaired_song_ini_file(
    path: &Path,
    contents: &str,
    keep_original_song_ini: bool,
) -> io::Result<()> {
    if keep_original_song_ini {
        backup_faulty_original_song_ini(path)?;
    }

    fs::write(path, contents)
}

fn backup_original_song_ini(path: &Path) -> io::Result<()> {
    let backup_path = original_song_ini_path(path);

    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(path, backup_path).map(|_| ())
}

fn backup_faulty_original_song_ini(path: &Path) -> io::Result<()> {
    let backup_path = faulty_original_song_ini_path(path);

    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(path, backup_path).map(|_| ())
}

fn original_song_ini_path(path: &Path) -> PathBuf {
    path.with_file_name("song.original.ini")
}

fn faulty_original_song_ini_path(path: &Path) -> PathBuf {
    path.with_file_name("song.original.faulty.ini")
}

fn parse_song_ini(relative_path: &str, contents: &str) -> Result<ParsedSongIni, String> {
    let parse_contents = contents.trim_start_matches('\u{feff}');

    validate_unique_ini_keys(relative_path, parse_contents)?;

    let ini = Ini::load_from_str(parse_contents)
        .map_err(|err| format!("Failed to parse {relative_path}: {err}"))?;
    let mut sections = Vec::new();

    for (name, properties) in ini.iter() {
        let entries = properties
            .iter()
            .map(|(key, value)| ParsedIniEntry {
                key: key.to_string(),
                value: value.to_string(),
            })
            .collect::<Vec<_>>();

        if name.is_none() && entries.is_empty() {
            continue;
        }

        sections.push(ParsedIniSection {
            name: name.map(str::to_string),
            entries,
        });
    }

    if sections.is_empty() {
        return Err(format!(
            "Failed to parse {relative_path}: no INI sections found"
        ));
    }

    validate_required_song_ini_fields(relative_path, &sections)?;

    Ok(ParsedSongIni {
        relative_path: relative_path.to_string(),
        sections,
    })
}

fn normalize_song_ini_key_case(contents: &str) -> String {
    let mut normalized_contents = String::with_capacity(contents.len());
    let mut current_section = String::new();

    for line in contents.split_inclusive('\n') {
        let (line_contents, line_ending) = split_line_ending(line);
        let line_without_bom = line_contents.trim_start_matches('\u{feff}');
        let trimmed_line = line_without_bom.trim();

        if trimmed_line.starts_with('[') && trimmed_line.ends_with(']') {
            current_section = trimmed_line[1..trimmed_line.len() - 1].trim().to_string();
            normalized_contents.push_str(line_contents);
            normalized_contents.push_str(line_ending);
            continue;
        }

        let Some(canonical_keys) = canonical_keys_for_section(&current_section) else {
            normalized_contents.push_str(line_contents);
            normalized_contents.push_str(line_ending);
            continue;
        };
        let Some((key_part, value_part)) = line_contents.split_once('=') else {
            normalized_contents.push_str(line_contents);
            normalized_contents.push_str(line_ending);
            continue;
        };
        let key = key_part.trim();
        let Some(canonical_key) = canonical_keys
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(key))
        else {
            normalized_contents.push_str(line_contents);
            normalized_contents.push_str(line_ending);
            continue;
        };

        if key == *canonical_key {
            normalized_contents.push_str(line_contents);
            normalized_contents.push_str(line_ending);
            continue;
        }

        let leading_whitespace_len = key_part.len() - key_part.trim_start().len();
        let trailing_whitespace_len = key_part.len() - key_part.trim_end().len();
        normalized_contents.push_str(&key_part[..leading_whitespace_len]);
        normalized_contents.push_str(canonical_key);
        normalized_contents.push_str(&key_part[key_part.len() - trailing_whitespace_len..]);
        normalized_contents.push('=');
        normalized_contents.push_str(value_part);
        normalized_contents.push_str(line_ending);
    }

    normalized_contents
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(line_without_newline) = line.strip_suffix('\n') {
        if let Some(line_without_crlf) = line_without_newline.strip_suffix('\r') {
            return (line_without_crlf, "\r\n");
        }

        return (line_without_newline, "\n");
    }

    (line, "")
}

fn update_song_ini_metadata_contents(
    contents: &str,
    metadata: &ScannedSongMetadataInput,
) -> String {
    let updates = [
        ("Artist", metadata.artist.as_str()),
        ("Title", metadata.title.as_str()),
        ("Year", metadata.year.as_str()),
        ("Genre", metadata.genre.as_str()),
        ("GameIcon", metadata.game_icon.as_str()),
    ];
    update_song_ini_values(contents, &updates)
}

fn update_song_ini_values(contents: &str, updates: &[(&str, &str)]) -> String {
    let mut updated_contents = String::with_capacity(contents.len());
    let mut in_song_info = false;
    let mut found_keys: HashSet<String> = HashSet::new();
    let append_line_ending = first_line_ending(contents).unwrap_or("\n");
    let mut appended_missing_keys = false;

    for line in contents.split_inclusive('\n') {
        let (line_contents, line_ending) = split_line_ending(line);
        let line_without_bom = line_contents.trim_start_matches('\u{feff}');
        let trimmed_line = line_without_bom.trim();

        if trimmed_line.starts_with('[') && trimmed_line.ends_with(']') {
            if in_song_info && !appended_missing_keys {
                append_missing_metadata_keys(
                    &mut updated_contents,
                    &updates,
                    &found_keys,
                    append_line_ending,
                );
                appended_missing_keys = true;
            }

            let section_name = trimmed_line[1..trimmed_line.len() - 1].trim();
            in_song_info = section_name == "SongInfo";
            updated_contents.push_str(line_contents);
            updated_contents.push_str(line_ending);
            continue;
        }

        if in_song_info {
            if let Some((key_part, _value_part)) = line_contents.split_once('=') {
                let key = key_part.trim();

                if let Some((canonical_key, value)) = updates
                    .iter()
                    .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
                {
                    if value.trim().is_empty() {
                        continue;
                    }
                    let leading_whitespace_len = key_part.len() - key_part.trim_start().len();
                    let trailing_whitespace_len = key_part.len() - key_part.trim_end().len();
                    updated_contents.push_str(&key_part[..leading_whitespace_len]);
                    updated_contents.push_str(canonical_key);
                    updated_contents
                        .push_str(&key_part[key_part.len() - trailing_whitespace_len..]);
                    updated_contents.push('=');
                    updated_contents.push_str(value);
                    updated_contents.push_str(line_ending);
                    found_keys.insert((*canonical_key).to_string());
                    continue;
                }
            }
        }

        updated_contents.push_str(line_contents);
        updated_contents.push_str(line_ending);
    }

    if in_song_info && !appended_missing_keys {
        append_missing_metadata_keys(
            &mut updated_contents,
            &updates,
            &found_keys,
            append_line_ending,
        );
    }

    updated_contents
}

fn remove_song_info_value(contents: &str, target_key: &str) -> String {
    let mut updated_contents = String::with_capacity(contents.len());
    let mut in_song_info = false;

    for line in contents.split_inclusive('\n') {
        let (line_contents, line_ending) = split_line_ending(line);
        let line_without_bom = line_contents.trim_start_matches('\u{feff}');
        let trimmed_line = line_without_bom.trim();

        if trimmed_line.starts_with('[') && trimmed_line.ends_with(']') {
            let section_name = trimmed_line[1..trimmed_line.len() - 1].trim();
            in_song_info = section_name == "SongInfo";
        } else if in_song_info
            && line_contents
                .split_once('=')
                .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case(target_key))
        {
            continue;
        }

        updated_contents.push_str(line_contents);
        updated_contents.push_str(line_ending);
    }

    updated_contents
}

fn append_missing_metadata_keys(
    contents: &mut String,
    updates: &[(&str, &str)],
    found_keys: &HashSet<String>,
    line_ending: &str,
) {
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push_str(line_ending);
    }

    for (key, value) in updates {
        if found_keys.contains(*key) || value.trim().is_empty() {
            continue;
        }

        contents.push_str(key);
        contents.push('=');
        contents.push_str(value);
        contents.push_str(line_ending);
    }
}

fn first_line_ending(contents: &str) -> Option<&'static str> {
    for line in contents.split_inclusive('\n') {
        let (_line_contents, line_ending) = split_line_ending(line);

        if !line_ending.is_empty() {
            return Some(if line_ending == "\r\n" { "\r\n" } else { "\n" });
        }
    }

    None
}

fn canonical_keys_for_section(section_name: &str) -> Option<&'static [&'static str]> {
    match section_name {
        "ModInfo" => Some(MOD_INFO_KEYS),
        "SongInfo" => Some(SONG_INFO_KEYS),
        _ => None,
    }
}

fn validate_required_song_ini_fields(
    relative_path: &str,
    sections: &[ParsedIniSection],
) -> Result<(), String> {
    if !sections
        .iter()
        .any(|section| section.name.as_deref() == Some("ModInfo"))
    {
        return Err(format!(
            "Failed to parse {relative_path}: missing required [ModInfo] section"
        ));
    }

    let Some(song_info) = sections
        .iter()
        .find(|section| section.name.as_deref() == Some("SongInfo"))
    else {
        return Err(format!(
            "Failed to parse {relative_path}: missing required [SongInfo] section"
        ));
    };

    let Some(checksum) = song_info
        .entries
        .iter()
        .find(|entry| entry.key == "Checksum")
    else {
        return Err(format!(
            "Failed to parse {relative_path}: missing required Checksum entry in [SongInfo]"
        ));
    };

    if checksum.value.trim().is_empty() {
        return Err(format!(
            "Failed to parse {relative_path}: Checksum entry in [SongInfo] must not be empty"
        ));
    }

    Ok(())
}

fn song_ini_checksum(song: &ParsedSongIni) -> Option<String> {
    song.sections
        .iter()
        .find(|section| section.name.as_deref() == Some("SongInfo"))?
        .entries
        .iter()
        .find(|entry| entry.key == "Checksum")
        .map(|entry| entry.value.trim().to_string())
}

fn scanned_songs(mods_dir: &Path, parsed_songs: &[ParsedSongIni]) -> Vec<ScannedSong> {
    let mut songs = parsed_songs
        .iter()
        .map(|song| scanned_song(mods_dir, song))
        .collect::<Vec<_>>();

    songs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    songs
}

fn scanned_song(mods_dir: &Path, song: &ParsedSongIni) -> ScannedSong {
    let song_ini_path = mods_dir.join(Path::new(&song.relative_path));
    let folder_absolute_path = song_ini_path
        .parent()
        .unwrap_or(mods_dir)
        .display()
        .to_string();
    let instruments = scanned_song_instruments(&song_ini_path, song);

    ScannedSong {
        relative_path: song.relative_path.clone(),
        folder_absolute_path,
        artist: song_info_value(song, "Artist"),
        title: song_info_value(song, "Title"),
        year: song_info_value(song, "Year"),
        genre: song_info_value(song, "Genre"),
        game_icon: song_info_value(song, "GameIcon"),
        has_original_song_ini: original_song_ini_path(&song_ini_path).is_file(),
        is_included: is_song_ini(&song_ini_path),
        instruments,
    }
}

fn scanned_song_instruments(song_ini_path: &Path, song: &ParsedSongIni) -> ScannedSongInstruments {
    read_instrument_sidecar(song_ini_path, song)
        .instruments
        .unwrap_or_else(unknown_instruments)
}

fn case_insensitive_child_dir(parent: &Path, expected_name: &str) -> Result<PathBuf, String> {
    let entries = fs::read_dir(parent)
        .map_err(|err| format!("Failed to read folder {}: {err}", parent.display()))?;
    let mut matches = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", parent.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.eq_ignore_ascii_case(expected_name) {
            matches.push(entry.path());
        }
    }

    if matches.is_empty() {
        return Err(format!(
            "Missing required {expected_name} folder in {}.",
            parent.display()
        ));
    }

    if matches.len() > 1 {
        return Err(format!(
            "Multiple folders match required {expected_name} folder casing in {}.",
            parent.display()
        ));
    }

    let path = matches.remove(0);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => Ok(path),
        Ok(_) => Err(format!("{expected_name} exists but is not a folder.")),
        Err(err) => Err(format!("Failed to inspect {}: {err}", path.display())),
    }
}

#[cfg(test)]
fn analyze_scanned_song_instruments_paths<F>(
    settings: &ScanSettings,
    mode: InstrumentAnalyzeMode,
    store: &SongIniStore,
    emit_progress: F,
) -> Result<InstrumentAnalyzeResult, String>
where
    F: FnMut(InstrumentAnalyzeProgress),
{
    let parsed_songs = stored_parsed_songs(store)?;
    analyze_scanned_song_instruments_for_songs(settings, mode, parsed_songs, emit_progress)
}

fn stored_parsed_songs(store: &SongIniStore) -> Result<Vec<ParsedSongIni>, String> {
    let songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    Ok(songs.clone())
}

fn analyze_scanned_song_instruments_for_songs<F>(
    settings: &ScanSettings,
    mode: InstrumentAnalyzeMode,
    parsed_songs: Vec<ParsedSongIni>,
    mut emit_progress: F,
) -> Result<InstrumentAnalyzeResult, String>
where
    F: FnMut(InstrumentAnalyzeProgress),
{
    if parsed_songs.is_empty() {
        return Err("Scan MODS folder before analyzing instruments.".to_string());
    }

    let candidates = parsed_songs
        .iter()
        .filter(|song| should_analyze_song_instruments(&settings.mods_dir, song, mode))
        .collect::<Vec<_>>();
    let total = candidates.len();
    let mut analyzed = 0;
    let mut errors = 0;
    let mode_value = instrument_analyze_mode_value(mode).to_string();
    let mut last_progress_emit = None;

    emit_instrument_analyze_progress(
        &mut emit_progress,
        0,
        total,
        "",
        &mode_value,
        &mut last_progress_emit,
        true,
    );

    for song in candidates {
        analyzed += 1;

        emit_instrument_analyze_progress(
            &mut emit_progress,
            analyzed,
            total,
            &song.relative_path,
            &mode_value,
            &mut last_progress_emit,
            false,
        );

        let song_ini_path = settings.mods_dir.join(Path::new(&song.relative_path));
        let instruments = analyze_and_write_instrument_sidecar(&song_ini_path, song)?;

        if instruments_have_error(&instruments) {
            errors += 1;
        }
    }

    emit_instrument_analyze_progress(
        &mut emit_progress,
        analyzed,
        total,
        "",
        &mode_value,
        &mut last_progress_emit,
        true,
    );

    Ok(InstrumentAnalyzeResult {
        songs: scanned_songs(&settings.mods_dir, &parsed_songs),
        analyzed,
        skipped: parsed_songs.len().saturating_sub(analyzed),
        errors,
    })
}

fn emit_instrument_analyze_progress<F>(
    emit_progress: &mut F,
    current: usize,
    total: usize,
    relative_path: &str,
    mode: &str,
    last_progress_emit: &mut Option<Instant>,
    force: bool,
) where
    F: FnMut(InstrumentAnalyzeProgress),
{
    if !force
        && last_progress_emit
            .map(|last_emit| last_emit.elapsed() < Duration::from_secs(1))
            .unwrap_or(false)
    {
        return;
    }

    emit_progress(InstrumentAnalyzeProgress {
        current,
        total,
        relative_path: relative_path.to_string(),
        mode: mode.to_string(),
    });
    *last_progress_emit = Some(Instant::now());
}

fn should_analyze_song_instruments(
    mods_dir: &Path,
    song: &ParsedSongIni,
    mode: InstrumentAnalyzeMode,
) -> bool {
    if mode == InstrumentAnalyzeMode::All {
        return true;
    }

    let song_ini_path = mods_dir.join(Path::new(&song.relative_path));
    let sidecar = read_instrument_sidecar(&song_ini_path, song);

    match mode {
        InstrumentAnalyzeMode::Missing => sidecar.instruments.is_none(),
        InstrumentAnalyzeMode::Errors => sidecar.has_fresh_error,
        InstrumentAnalyzeMode::All => true,
    }
}

fn analyze_and_write_instrument_sidecar(
    song_ini_path: &Path,
    song: &ParsedSongIni,
) -> Result<ScannedSongInstruments, String> {
    let Some(checksum) = song_ini_checksum(song) else {
        let instruments = error_instruments(vec!["Missing checksum.".to_string()]);
        write_instrument_sidecar(
            song_ini_path,
            "",
            &PakIdentity::error("", "Missing checksum.".to_string()),
            &instruments,
        )?;
        return Ok(instruments);
    };
    let identity = current_pak_identity(song_ini_path, &checksum);
    let instruments = if identity.status == "present" {
        match resolved_pak_path(song_ini_path, &identity.relative_path) {
            Some(pak_path) => match song_pak_analyzer::analyze_song_pak_instruments(
                &pak_path.to_string_lossy(),
                &checksum,
            ) {
                Ok(availability) => instruments_from_availability(availability),
                Err(err) => error_instruments(vec![err]),
            },
            None => error_instruments(vec!["Song folder could not be determined.".to_string()]),
        }
    } else {
        error_instruments(vec![identity
            .error
            .clone()
            .unwrap_or_else(|| "Pak file could not be resolved.".to_string())])
    };

    write_instrument_sidecar(song_ini_path, &checksum, &identity, &instruments)?;
    Ok(instruments)
}

fn read_instrument_sidecar(song_ini_path: &Path, song: &ParsedSongIni) -> SidecarReadResult {
    let Some(checksum) = song_ini_checksum(song) else {
        return SidecarReadResult {
            instruments: None,
            has_fresh_error: false,
        };
    };
    let sidecar_path = instrument_sidecar_path(song_ini_path);
    let contents = match fs::read_to_string(&sidecar_path) {
        Ok(contents) => contents,
        Err(_) => {
            return SidecarReadResult {
                instruments: None,
                has_fresh_error: false,
            };
        }
    };
    let ini = match Ini::load_from_str(&contents) {
        Ok(ini) => ini,
        Err(_) => {
            return SidecarReadResult {
                instruments: None,
                has_fresh_error: false,
            };
        }
    };
    let identity = current_pak_identity(song_ini_path, &checksum);

    if ini_string(&ini, "Cache", "AnalyzerVersion").as_deref() != Some(INSTRUMENT_ANALYZER_VERSION)
        || ini_string(&ini, "Cache", "Checksum").as_deref() != Some(checksum.as_str())
        || ini_string(&ini, "Cache", "PakStatus").as_deref() != Some(identity.status.as_str())
        || ini_string(&ini, "Cache", "PakRelativePath").as_deref()
            != Some(identity.relative_path.as_str())
        || ini_string(&ini, "Cache", "PakSize").as_deref()
            != Some(identity.size_string().as_deref().unwrap_or(""))
        || ini_string(&ini, "Cache", "PakModifiedMillis").as_deref()
            != Some(identity.modified_string().as_deref().unwrap_or(""))
        || ini_string(&ini, "Cache", "PakError").as_deref()
            != Some(identity.error.as_deref().unwrap_or(""))
    {
        return SidecarReadResult {
            instruments: None,
            has_fresh_error: false,
        };
    }

    let Some(instruments) = instruments_from_sidecar(&ini) else {
        return SidecarReadResult {
            instruments: None,
            has_fresh_error: false,
        };
    };
    let has_fresh_error = instruments_have_error(&instruments);

    SidecarReadResult {
        instruments: Some(instruments),
        has_fresh_error,
    }
}

fn write_instrument_sidecar(
    song_ini_path: &Path,
    checksum: &str,
    identity: &PakIdentity,
    instruments: &ScannedSongInstruments,
) -> Result<(), String> {
    let sidecar_path = instrument_sidecar_path(song_ini_path);
    let contents = write_instrument_sidecar_contents(checksum, identity, instruments);

    fs::write(&sidecar_path, contents)
        .map_err(|err| format!("Failed to write {}: {err}", sidecar_path.display()))
}

fn write_instrument_sidecar_contents(
    checksum: &str,
    identity: &PakIdentity,
    instruments: &ScannedSongInstruments,
) -> String {
    let mut contents = String::new();

    contents.push_str("[Cache]\n");
    push_ini_entry(
        &mut contents,
        "AnalyzerVersion",
        INSTRUMENT_ANALYZER_VERSION,
    );
    push_ini_entry(&mut contents, "Checksum", checksum);
    push_ini_entry(&mut contents, "PakStatus", &identity.status);
    push_ini_entry(&mut contents, "PakRelativePath", &identity.relative_path);
    push_ini_entry(
        &mut contents,
        "PakSize",
        identity.size_string().as_deref().unwrap_or(""),
    );
    push_ini_entry(
        &mut contents,
        "PakModifiedMillis",
        identity.modified_string().as_deref().unwrap_or(""),
    );
    push_ini_entry(
        &mut contents,
        "PakError",
        identity.error.as_deref().unwrap_or(""),
    );

    push_instrument_sidecar_section(&mut contents, "Guitar", &instruments.guitar);
    push_instrument_sidecar_section(&mut contents, "Bass", &instruments.bass);
    push_instrument_sidecar_section(&mut contents, "Drums", &instruments.drums);
    push_instrument_sidecar_section(&mut contents, "Vocals", &instruments.vocals);
    push_instrument_sidecar_section(&mut contents, "CoopGuitar", &instruments.coop_guitar);
    push_instrument_sidecar_section(&mut contents, "CoopBass", &instruments.coop_bass);

    contents
}

fn push_instrument_sidecar_section(
    contents: &mut String,
    section: &str,
    instrument: &InstrumentColumnSummary,
) {
    contents.push('\n');
    contents.push('[');
    contents.push_str(section);
    contents.push_str("]\n");
    push_ini_entry(contents, "Value", &instrument.value);
    push_ini_entry(contents, "Easy", bool_ini_value(instrument.easy));
    push_ini_entry(contents, "Medium", bool_ini_value(instrument.medium));
    push_ini_entry(contents, "Hard", bool_ini_value(instrument.hard));
    push_ini_entry(contents, "Expert", bool_ini_value(instrument.expert));
    push_ini_entry(contents, "Tooltip", &json_string(&instrument.tooltip));
    push_ini_entry(contents, "Errors", &json_string(&instrument.errors));
}

fn push_ini_entry(contents: &mut String, key: &str, value: &str) {
    contents.push_str(key);
    contents.push('=');
    contents.push_str(value);
    contents.push('\n');
}

fn instruments_from_sidecar(ini: &Ini) -> Option<ScannedSongInstruments> {
    Some(ScannedSongInstruments {
        guitar: instrument_column_from_sidecar(ini, "Guitar")?,
        bass: instrument_column_from_sidecar(ini, "Bass")?,
        drums: instrument_column_from_sidecar(ini, "Drums")?,
        vocals: instrument_column_from_sidecar(ini, "Vocals")?,
        coop_guitar: instrument_column_from_sidecar(ini, "CoopGuitar")?,
        coop_bass: instrument_column_from_sidecar(ini, "CoopBass")?,
    })
}

fn instrument_column_from_sidecar(ini: &Ini, section: &str) -> Option<InstrumentColumnSummary> {
    let value = ini_string(ini, section, "Value")?;

    if !is_known_instrument_value(&value) || value == "Unknown" {
        return None;
    }

    let tooltip = ini_string(ini, section, "Tooltip")
        .and_then(|value| json_value::<String>(&value).or(Some(value)))?;
    let errors = json_value::<Vec<String>>(&ini_string(ini, section, "Errors")?)?;

    Some(InstrumentColumnSummary {
        value,
        tooltip,
        easy: ini_bool(ini, section, "Easy")?,
        medium: ini_bool(ini, section, "Medium")?,
        hard: ini_bool(ini, section, "Hard")?,
        expert: ini_bool(ini, section, "Expert")?,
        errors,
    })
}

fn current_pak_identity(song_ini_path: &Path, checksum: &str) -> PakIdentity {
    let Some(song_dir) = song_ini_path.parent() else {
        return PakIdentity::error("", "Song folder could not be determined.".to_string());
    };
    let content_dir = match case_insensitive_child_dir(song_dir, "Content") {
        Ok(content_dir) => content_dir,
        Err(err) => {
            return PakIdentity::error(&format!("Content/a{checksum}_song.pak.xen"), err);
        }
    };
    let expected_pak_path = content_dir.join(format!("a{checksum}_song.pak.xen"));

    match case_insensitive_child_paths(&expected_pak_path) {
        Ok(matches) if matches.is_empty() => PakIdentity::error(
            &relative_path_string(song_dir, &expected_pak_path),
            format!("Missing required pak file {}.", expected_pak_path.display()),
        ),
        Ok(matches) if matches.len() > 1 => PakIdentity::error(
            &relative_path_string(song_dir, &expected_pak_path),
            format!(
                "Multiple files match required pak file {}.",
                expected_pak_path.display()
            ),
        ),
        Ok(mut matches) => {
            let path = matches.remove(0);
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => PakIdentity {
                    status: "present".to_string(),
                    relative_path: relative_path_string(song_dir, &path),
                    size: Some(metadata.len()),
                    modified_millis: metadata
                        .modified()
                        .ok()
                        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                        .map(|duration| duration.as_millis()),
                    error: None,
                },
                Ok(_) => PakIdentity::error(
                    &relative_path_string(song_dir, &path),
                    format!("Expected pak path is not a file: {}", path.display()),
                ),
                Err(err) => PakIdentity::error(
                    &relative_path_string(song_dir, &path),
                    format!("Failed to inspect {}: {err}", path.display()),
                ),
            }
        }
        Err(err) => PakIdentity::error(
            &relative_path_string(song_dir, &expected_pak_path),
            format!(
                "Failed to inspect pak file {}: {err}",
                expected_pak_path.display()
            ),
        ),
    }
}

fn resolved_pak_path(song_ini_path: &Path, relative_path: &str) -> Option<PathBuf> {
    song_ini_path
        .parent()
        .map(|song_dir| song_dir.join(Path::new(relative_path)))
}

fn instrument_sidecar_path(song_ini_path: &Path) -> PathBuf {
    song_ini_path.with_file_name(INSTRUMENT_SIDECAR_FILE_NAME)
}

fn unknown_instruments() -> ScannedSongInstruments {
    ScannedSongInstruments {
        guitar: unknown_column("Guitar"),
        bass: unknown_column("Bass"),
        drums: unknown_column("Drums"),
        vocals: unknown_column("Vocals"),
        coop_guitar: unknown_column("CoopGuitar"),
        coop_bass: unknown_column("CoopBass"),
    }
}

fn unknown_column(label: &str) -> InstrumentColumnSummary {
    InstrumentColumnSummary {
        value: "Unknown".to_string(),
        tooltip: format!("{label}: instrument data has not been analyzed."),
        easy: false,
        medium: false,
        hard: false,
        expert: false,
        errors: Vec::new(),
    }
}

fn instruments_have_error(instruments: &ScannedSongInstruments) -> bool {
    [
        &instruments.guitar,
        &instruments.bass,
        &instruments.drums,
        &instruments.vocals,
        &instruments.coop_guitar,
        &instruments.coop_bass,
    ]
    .iter()
    .any(|instrument| instrument.value == "Error")
}

fn instrument_analyze_mode_value(mode: InstrumentAnalyzeMode) -> &'static str {
    match mode {
        InstrumentAnalyzeMode::Missing => "missing",
        InstrumentAnalyzeMode::Errors => "errors",
        InstrumentAnalyzeMode::All => "all",
    }
}

fn is_known_instrument_value(value: &str) -> bool {
    matches!(
        value,
        "Unknown" | "No" | "Easy" | "Medium" | "Hard" | "Expert" | "Error"
    )
}

fn bool_ini_value(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn ini_bool(ini: &Ini, section: &str, key: &str) -> Option<bool> {
    match ini_string(ini, section, key)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn ini_string(ini: &Ini, section: &str, key: &str) -> Option<String> {
    ini.section(Some(section))?
        .get(key)
        .map(|value| value.trim().to_string())
}

fn json_string<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn json_value<T>(value: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(value).ok()
}

fn relative_path_string(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .ok()
        .map(|relative_path| {
            relative_path
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|| path.display().to_string())
}

impl PakIdentity {
    fn error(relative_path: &str, error: String) -> Self {
        Self {
            status: "error".to_string(),
            relative_path: relative_path.to_string(),
            size: None,
            modified_millis: None,
            error: Some(error),
        }
    }

    fn size_string(&self) -> Option<String> {
        self.size.map(|size| size.to_string())
    }

    fn modified_string(&self) -> Option<String> {
        self.modified_millis
            .map(|modified_millis| modified_millis.to_string())
    }
}

fn instruments_from_availability(
    availability: song_pak_analyzer::SongPakInstrumentAvailability,
) -> ScannedSongInstruments {
    let errors = availability.errors;

    ScannedSongInstruments {
        guitar: difficulty_column("Guitar", availability.guitar, &errors),
        bass: difficulty_column("Bass", availability.bass, &errors),
        drums: difficulty_column("Drums", availability.drums, &errors),
        vocals: vocals_column(availability.vocals.supported, &errors),
        coop_guitar: difficulty_column("CoopGuitar", availability.coop_guitar, &errors),
        coop_bass: difficulty_column("CoopBass", availability.coop_bass, &errors),
    }
}

fn error_instruments(errors: Vec<String>) -> ScannedSongInstruments {
    ScannedSongInstruments {
        guitar: empty_column("Guitar", &errors),
        bass: empty_column("Bass", &errors),
        drums: empty_column("Drums", &errors),
        vocals: empty_column("Vocals", &errors),
        coop_guitar: empty_column("CoopGuitar", &errors),
        coop_bass: empty_column("CoopBass", &errors),
    }
}

fn difficulty_column(
    label: &str,
    availability: song_pak_analyzer::DifficultyAvailability,
    errors: &[String],
) -> InstrumentColumnSummary {
    instrument_column(
        label,
        availability.easy,
        availability.medium,
        availability.hard,
        availability.expert,
        errors,
    )
}

fn vocals_column(supported: bool, errors: &[String]) -> InstrumentColumnSummary {
    instrument_column("Vocals", false, false, false, supported, errors)
}

fn empty_column(label: &str, errors: &[String]) -> InstrumentColumnSummary {
    instrument_column(label, false, false, false, false, errors)
}

fn instrument_column(
    label: &str,
    easy: bool,
    medium: bool,
    hard: bool,
    expert: bool,
    errors: &[String],
) -> InstrumentColumnSummary {
    let max_level = max_instrument_level(easy, medium, hard, expert);
    let value = if max_level == "No" && !errors.is_empty() {
        "Error".to_string()
    } else {
        max_level.to_string()
    };
    let tooltip = if value == "Error" {
        errors.join("\n")
    } else {
        format!(
            "{label}: Easy {}, Medium {}, Hard {}, Expert {}",
            level_state(easy),
            level_state(medium),
            level_state(hard),
            level_state(expert)
        )
    };

    InstrumentColumnSummary {
        value,
        tooltip,
        easy,
        medium,
        hard,
        expert,
        errors: errors.to_vec(),
    }
}

fn max_instrument_level(easy: bool, medium: bool, hard: bool, expert: bool) -> &'static str {
    if expert {
        "Expert"
    } else if hard {
        "Hard"
    } else if medium {
        "Medium"
    } else if easy {
        "Easy"
    } else {
        "No"
    }
}

fn level_state(supported: bool) -> &'static str {
    if supported {
        "Yes"
    } else {
        "No"
    }
}

fn song_info_value(song: &ParsedSongIni, key: &str) -> String {
    song.sections
        .iter()
        .find(|section| section.name.as_deref() == Some("SongInfo"))
        .and_then(|section| {
            section
                .entries
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| entry.value.trim().to_string())
        })
        .unwrap_or_default()
}

fn duplicate_checksum_groups(parsed_songs: &[ParsedSongIni]) -> Vec<DuplicateChecksumGroup> {
    let mut paths_by_checksum: HashMap<String, Vec<String>> = HashMap::new();

    for song in parsed_songs {
        let Some(checksum) = song_ini_checksum(song) else {
            continue;
        };

        paths_by_checksum
            .entry(checksum)
            .or_default()
            .push(song.relative_path.clone());
    }

    let mut groups = paths_by_checksum
        .into_iter()
        .filter_map(|(checksum, mut relative_paths)| {
            if relative_paths.len() < 2 {
                return None;
            }

            relative_paths.sort();
            Some(DuplicateChecksumGroup {
                checksum,
                relative_paths,
            })
        })
        .collect::<Vec<_>>();

    groups.sort_by(|left, right| left.checksum.cmp(&right.checksum));
    groups
}

fn song_ini_folder_conflicts(mods_dir: &Path) -> Result<Vec<SongIniFolderConflict>, String> {
    let mut variant_paths = Vec::new();
    collect_song_ini_variant_files(mods_dir, &mut variant_paths)?;
    let mut paths_by_folder = HashMap::<String, Vec<String>>::new();

    for path in variant_paths {
        let folder = mods_relative_path(mods_dir, path.parent().unwrap_or(mods_dir))?;
        paths_by_folder
            .entry(if folder.is_empty() {
                ".".to_string()
            } else {
                folder
            })
            .or_default()
            .push(mods_relative_path(mods_dir, &path)?);
    }

    let mut conflicts = paths_by_folder
        .into_iter()
        .filter_map(|(folder_path, mut file_paths)| {
            if file_paths.len() < 2 {
                return None;
            }

            file_paths.sort_by_key(|path| song_ini_variant_sort_key(path));
            Some(SongIniFolderConflict {
                folder_path,
                file_paths,
            })
        })
        .collect::<Vec<_>>();
    conflicts.sort_by(|left, right| left.folder_path.cmp(&right.folder_path));
    Ok(conflicts)
}

fn collect_song_ini_variant_files(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            collect_song_ini_variant_files(&path, paths)?;
        } else if file_type.is_file() && (is_scanned_song_ini(&path) || is_disabled_song_ini(&path))
        {
            paths.push(path);
        }
    }

    Ok(())
}

fn song_ini_variant_sort_key(path: &str) -> (u8, String) {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let rank = if file_name.eq_ignore_ascii_case("song.ini") {
        0
    } else if file_name.eq_ignore_ascii_case("song.excluded.ini") {
        1
    } else {
        2
    };

    (rank, path.to_string())
}

fn find_sibling_disabled_song_ini(song_ini_path: &Path) -> Result<Option<PathBuf>, String> {
    let Some(parent) = song_ini_path.parent() else {
        return Ok(None);
    };

    let entries = fs::read_dir(parent)
        .map_err(|err| format!("Failed to read folder {}: {err}", parent.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", parent.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_file() && is_disabled_song_ini(&path) {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn song_content_issues(
    mods_dir: &Path,
    parsed_songs: &[ParsedSongIni],
) -> Result<Vec<SongContentIssue>, String> {
    song_content_issues_with_progress(mods_dir, parsed_songs, |_, _, _| {})
}

fn song_content_issues_with_progress<F>(
    mods_dir: &Path,
    parsed_songs: &[ParsedSongIni],
    mut emit_progress: F,
) -> Result<Vec<SongContentIssue>, String>
where
    F: FnMut(usize, usize, &str),
{
    let mut issues = Vec::new();

    for (index, song) in parsed_songs.iter().enumerate() {
        emit_progress(index + 1, parsed_songs.len(), &song.relative_path);
        let Some(checksum) = song_ini_checksum(song) else {
            continue;
        };
        let song_ini_path = mods_dir.join(Path::new(&song.relative_path));
        let song_ini_absolute_path = song_ini_path.display().to_string();
        let Some(song_dir) = song_ini_path.parent() else {
            continue;
        };

        let Some(content_dir) = normalized_child_dir(
            &mut issues,
            song,
            &song_ini_absolute_path,
            &checksum,
            song_dir,
            "Content",
        )?
        else {
            continue;
        };

        validate_required_content_file(
            &mut issues,
            song,
            &song_ini_absolute_path,
            &checksum,
            &content_dir.join(format!("a{checksum}_song.pak.xen")),
        );

        // Temporary dev-only shortcut: local debug runs may omit bulky MUSIC assets.
        let suppress_missing_music_folder = cfg!(debug_assertions);
        let Some(music_dir) = normalized_child_dir_with_missing_issue(
            &mut issues,
            song,
            &song_ini_absolute_path,
            &checksum,
            &content_dir,
            "MUSIC",
            !suppress_missing_music_folder,
        )?
        else {
            validate_extra_content_entries(
                &mut issues,
                song,
                &song_ini_absolute_path,
                &checksum,
                &content_dir,
                &[format!("a{checksum}_song.pak.xen"), "MUSIC".to_string()],
            )?;
            continue;
        };

        let music_file_names = [
            format!("{checksum}_preview.fsb.xen"),
            format!("{checksum}_1.fsb.xen"),
            format!("{checksum}_2.fsb.xen"),
            format!("{checksum}_3.fsb.xen"),
        ];

        for file_name in &music_file_names {
            validate_required_content_file(
                &mut issues,
                song,
                &song_ini_absolute_path,
                &checksum,
                &music_dir.join(file_name),
            );
        }

        validate_extra_content_entries(
            &mut issues,
            song,
            &song_ini_absolute_path,
            &checksum,
            &content_dir,
            &[format!("a{checksum}_song.pak.xen"), "MUSIC".to_string()],
        )?;
        validate_extra_content_entries(
            &mut issues,
            song,
            &song_ini_absolute_path,
            &checksum,
            &music_dir,
            &music_file_names,
        )?;
    }

    issues.sort_by(|left, right| {
        left.song_ini_relative_path
            .cmp(&right.song_ini_relative_path)
            .then(left.absolute_path.cmp(&right.absolute_path))
            .then(left.message.cmp(&right.message))
    });
    Ok(issues)
}

fn song_scan_progress(
    phase: &str,
    current: usize,
    total: usize,
    relative_path: &str,
) -> SongScanProgress {
    SongScanProgress {
        phase: phase.to_string(),
        current,
        total,
        relative_path: relative_path.to_string(),
    }
}

fn normalized_child_dir(
    issues: &mut Vec<SongContentIssue>,
    song: &ParsedSongIni,
    song_ini_absolute_path: &str,
    checksum: &str,
    parent: &Path,
    expected_name: &str,
) -> Result<Option<PathBuf>, String> {
    normalized_child_dir_with_missing_issue(
        issues,
        song,
        song_ini_absolute_path,
        checksum,
        parent,
        expected_name,
        true,
    )
}

fn normalized_child_dir_with_missing_issue(
    issues: &mut Vec<SongContentIssue>,
    song: &ParsedSongIni,
    song_ini_absolute_path: &str,
    checksum: &str,
    parent: &Path,
    expected_name: &str,
    report_missing: bool,
) -> Result<Option<PathBuf>, String> {
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                format!("Failed to read folder {}: {err}", parent.display()),
                parent,
            );
            return Ok(None);
        }
    };
    let mut matches = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                push_content_issue(
                    issues,
                    song,
                    song_ini_absolute_path,
                    checksum,
                    format!("Failed to read entry in {}: {err}", parent.display()),
                    parent,
                );
                return Ok(None);
            }
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.eq_ignore_ascii_case(expected_name) {
            matches.push(entry.path());
        }
    }

    if matches.is_empty() {
        if report_missing {
            let expected_path = parent.join(expected_name);
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                format!("Missing required {expected_name} folder."),
                &expected_path,
            );
        }
        return Ok(None);
    }

    if matches.len() > 1 {
        push_content_issue(
            issues,
            song,
            song_ini_absolute_path,
            checksum,
            format!("Multiple folders match required {expected_name} folder casing."),
            parent,
        );
        return Ok(None);
    }

    let path = matches.remove(0);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(err) => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                format!("Failed to inspect {}: {err}", path.display()),
                &path,
            );
            return Ok(None);
        }
    };

    if !metadata.is_dir() {
        push_content_issue(
            issues,
            song,
            song_ini_absolute_path,
            checksum,
            format!("{expected_name} exists but is not a folder."),
            &path,
        );
        return Ok(None);
    }

    Ok(Some(path))
}

fn validate_required_content_file(
    issues: &mut Vec<SongContentIssue>,
    song: &ParsedSongIni,
    song_ini_absolute_path: &str,
    checksum: &str,
    path: &Path,
) {
    let path = match case_insensitive_child_paths(path) {
        Ok(matches) if matches.is_empty() => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                "Missing required file.".to_string(),
                path,
            );
            return;
        }
        Ok(matches) if matches.len() > 1 => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                "Multiple files match required file casing.".to_string(),
                path,
            );
            return;
        }
        Ok(mut matches) => matches.remove(0),
        Err(err) => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                format!("Failed to inspect file: {err}"),
                path,
            );
            return;
        }
    };

    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => push_content_issue(
            issues,
            song,
            song_ini_absolute_path,
            checksum,
            "Expected file is not a file.".to_string(),
            &path,
        ),
        Err(err) => push_content_issue(
            issues,
            song,
            song_ini_absolute_path,
            checksum,
            format!("Failed to inspect file: {err}"),
            &path,
        ),
    }
}

fn case_insensitive_child_paths(path: &Path) -> io::Result<Vec<PathBuf>> {
    let Some(parent) = path.parent() else {
        return Ok(Vec::new());
    };
    let Some(expected_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(Vec::new());
    };

    let mut matches = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.eq_ignore_ascii_case(expected_name) {
            matches.push(entry.path());
        }
    }

    Ok(matches)
}

fn validate_extra_content_entries(
    issues: &mut Vec<SongContentIssue>,
    song: &ParsedSongIni,
    song_ini_absolute_path: &str,
    checksum: &str,
    dir: &Path,
    allowed_names: &[String],
) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                format!("Failed to read folder {}: {err}", dir.display()),
                dir,
            );
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                push_content_issue(
                    issues,
                    song,
                    song_ini_absolute_path,
                    checksum,
                    format!("Failed to read entry in {}: {err}", dir.display()),
                    dir,
                );
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();

        if !allowed_names
            .iter()
            .any(|allowed_name| name.eq_ignore_ascii_case(allowed_name))
        {
            push_content_issue(
                issues,
                song,
                song_ini_absolute_path,
                checksum,
                "Unexpected file or folder.".to_string(),
                &entry.path(),
            );
        }
    }

    Ok(())
}

fn push_content_issue(
    issues: &mut Vec<SongContentIssue>,
    song: &ParsedSongIni,
    song_ini_absolute_path: &str,
    checksum: &str,
    message: String,
    path: &Path,
) {
    issues.push(SongContentIssue {
        song_ini_relative_path: song.relative_path.clone(),
        song_ini_absolute_path: song_ini_absolute_path.to_string(),
        checksum: checksum.to_string(),
        message,
        absolute_path: path.display().to_string(),
    });
}

fn validate_unique_ini_keys(relative_path: &str, contents: &str) -> Result<(), String> {
    let mut current_section = String::new();
    let mut keys_by_section: Vec<(String, HashSet<String>)> =
        vec![(current_section.clone(), HashSet::new())];

    for (line_index, line) in contents.lines().enumerate() {
        let trimmed_line = line.trim();

        if trimmed_line.is_empty() || trimmed_line.starts_with(';') || trimmed_line.starts_with('#')
        {
            continue;
        }

        if trimmed_line.starts_with('[') && trimmed_line.ends_with(']') {
            current_section = trimmed_line[1..trimmed_line.len() - 1].trim().to_string();

            if !keys_by_section
                .iter()
                .any(|(section, _)| section == &current_section)
            {
                keys_by_section.push((current_section.clone(), HashSet::new()));
            }

            continue;
        }

        let Some((key, _)) = trimmed_line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();

        if key.is_empty() {
            continue;
        }

        let Some((_, section_keys)) = keys_by_section
            .iter_mut()
            .find(|(section, _)| section == &current_section)
        else {
            continue;
        };

        if !section_keys.insert(key.clone()) {
            let section_label = if current_section.is_empty() {
                "global section".to_string()
            } else {
                format!("[{current_section}]")
            };

            return Err(format!(
                "Failed to parse {relative_path}: duplicate key `{key}` in {section_label} on line {}",
                line_index + 1
            ));
        }
    }

    Ok(())
}

fn replace_song_ini_store(
    store: &SongIniStore,
    parsed_songs: Vec<ParsedSongIni>,
) -> Result<(), String> {
    let mut songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    *songs = parsed_songs;
    Ok(())
}

fn upsert_song_ini_store(
    store: &SongIniStore,
    parsed_song: ParsedSongIni,
) -> Result<Vec<ParsedSongIni>, String> {
    let mut songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    if let Some(existing_song) = songs
        .iter_mut()
        .find(|song| song.relative_path == parsed_song.relative_path)
    {
        *existing_song = parsed_song;
    } else {
        songs.push(parsed_song);
    }

    Ok(songs.clone())
}

fn remove_song_ini_from_store(store: &SongIniStore, relative_path: &str) -> Result<usize, String> {
    Ok(remove_song_ini_from_store_with_songs(store, relative_path)?.len())
}

fn remove_song_ini_from_store_with_songs(
    store: &SongIniStore,
    relative_path: &str,
) -> Result<Vec<ParsedSongIni>, String> {
    let mut songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    songs.retain(|song| song.relative_path != relative_path);
    Ok(songs.clone())
}

fn store_song_count(store: &SongIniStore) -> Result<usize, String> {
    let songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    Ok(songs.len())
}

fn checked_mods_relative_path(mods_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let full_path = checked_mods_relative_path_allow_missing(mods_dir, relative_path)?;

    if !full_path.is_file() {
        return Err(format!(
            "Skipped {} because it is not a file.",
            full_path.display()
        ));
    }

    let canonical_path = full_path
        .canonicalize()
        .map(normalize_settings_path)
        .map_err(|err| format!("Failed to resolve {}: {err}", full_path.display()))?;

    if !canonical_path.starts_with(mods_dir) {
        return Err(format!("Rejected path outside MODS: {relative_path}."));
    }

    Ok(canonical_path)
}

fn checked_mods_relative_path_allow_missing(
    mods_dir: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let path = Path::new(relative_path);

    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "Rejected unsafe MODS-relative path {relative_path}."
        ));
    }

    let full_path = mods_dir.join(path);
    let Some(parent_path) = full_path.parent() else {
        return Ok(full_path);
    };

    if !parent_path.exists() {
        return Ok(full_path);
    }

    let canonical_parent_path = parent_path
        .canonicalize()
        .map(normalize_settings_path)
        .map_err(|err| format!("Failed to resolve {}: {err}", parent_path.display()))?;

    if !canonical_parent_path.starts_with(mods_dir) {
        return Err(format!("Rejected path outside MODS: {relative_path}."));
    }

    let Some(file_name) = full_path.file_name() else {
        return Err(format!(
            "Rejected unsafe MODS-relative path {relative_path}."
        ));
    };

    Ok(canonical_parent_path.join(file_name))
}

fn keep_patterns(pattern: &str) -> Vec<String> {
    pattern
        .split(',')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect()
}

fn matches_keep_patterns(path: &Path, keep_patterns: &[String]) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();

    keep_patterns
        .iter()
        .any(|pattern| wildcard_match(pattern, &file_name))
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let mut pattern_index = 0;
    let mut text_index = 0;
    let mut last_star = None;
    let mut retry_text_index = 0;

    while text_index < text.len() {
        if pattern_index < pattern.len() && pattern[pattern_index] == text[text_index] {
            pattern_index += 1;
            text_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            last_star = Some(pattern_index);
            pattern_index += 1;
            retry_text_index = text_index;
        } else if let Some(star_index) = last_star {
            pattern_index = star_index + 1;
            retry_text_index += 1;
            text_index = retry_text_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn mods_relative_path(mods_dir: &Path, path: &Path) -> Result<String, String> {
    let relative_path = path.strip_prefix(mods_dir).map_err(|err| {
        format!(
            "Failed to build relative path for {} from {}: {err}",
            path.display(),
            mods_dir.display()
        )
    })?;
    let path_parts = relative_path
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>();

    Ok(path_parts.join("/"))
}

fn find_song_ini_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut song_ini_paths = Vec::new();
    collect_song_ini_files(mods_dir, &mut song_ini_paths)?;
    song_ini_paths.sort();
    Ok(song_ini_paths)
}

fn find_scanned_song_ini_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut song_ini_paths = find_song_ini_files(mods_dir)?;
    song_ini_paths.extend(find_excluded_song_ini_files(mods_dir)?);
    song_ini_paths.sort();
    Ok(song_ini_paths)
}

fn find_excluded_song_ini_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut excluded_song_ini_paths = Vec::new();
    collect_excluded_song_ini_files(mods_dir, &mut excluded_song_ini_paths)?;
    excluded_song_ini_paths.sort();
    Ok(excluded_song_ini_paths)
}

fn find_instrument_sidecar_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut instrument_sidecar_paths = Vec::new();
    collect_instrument_sidecar_files(mods_dir, &mut instrument_sidecar_paths)?;
    instrument_sidecar_paths.sort();
    Ok(instrument_sidecar_paths)
}

fn collect_song_ini_files(dir: &Path, song_ini_paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            collect_song_ini_files(&path, song_ini_paths)?;
        } else if file_type.is_file() && is_song_ini(&path) {
            song_ini_paths.push(path);
        }
    }

    Ok(())
}

fn collect_excluded_song_ini_files(
    dir: &Path,
    excluded_song_ini_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            collect_excluded_song_ini_files(&path, excluded_song_ini_paths)?;
        } else if file_type.is_file() && is_excluded_song_ini(&path) {
            excluded_song_ini_paths.push(path);
        }
    }

    Ok(())
}

fn collect_instrument_sidecar_files(
    dir: &Path,
    instrument_sidecar_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read folder {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            collect_instrument_sidecar_files(&path, instrument_sidecar_paths)?;
        } else if file_type.is_file() && is_instrument_sidecar(&path) {
            instrument_sidecar_paths.push(path);
        }
    }

    Ok(())
}

fn is_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.ini"))
}

fn is_excluded_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.excluded.ini"))
}

fn is_scanned_song_ini(path: &Path) -> bool {
    is_song_ini(path) || is_excluded_song_ini(path)
}

fn is_instrument_sidecar(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(INSTRUMENT_SIDECAR_FILE_NAME))
}

fn is_disabled_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.disabled.ini"))
}

fn read_project_settings_from_ini(contents: &str) -> StoredProjectSettings {
    let mut settings = StoredProjectSettings::default();
    let mut in_project_section = false;

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_project_section = &line[1..line.len() - 1] == "project";
            continue;
        }

        if !in_project_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unescape_ini_value(value.trim());

        match key.trim() {
            "disclaimer_accepted" => {
                settings.disclaimer_accepted = parse_ini_bool(&value).unwrap_or(false)
            }
            "mods_dir" => {
                settings.mods_dir =
                    (!value.is_empty()).then(|| normalize_settings_path_string(value));
            }
            "official_gamelogos_dir" => {
                settings.official_gamelogos_dir =
                    (!value.is_empty()).then(|| normalize_settings_path_string(value))
            }
            "keep_original_song_ini" => {
                settings.keep_original_song_ini = parse_ini_bool(&value).unwrap_or(true)
            }
            "check_for_updates_on_startup" => {
                settings.check_for_updates_on_startup = parse_ini_bool(&value).unwrap_or(true)
            }
            _ => {}
        }
    }

    settings
}

fn write_project_settings_to_ini(settings: &StoredProjectSettings) -> String {
    format!(
        "[project]\ndisclaimer_accepted={}\nmods_dir={}\nofficial_gamelogos_dir={}\nkeep_original_song_ini={}\ncheck_for_updates_on_startup={}\n",
        settings.disclaimer_accepted,
        settings.mods_dir.as_deref().unwrap_or(""),
        settings.official_gamelogos_dir.as_deref().unwrap_or(""),
        settings.keep_original_song_ini,
        settings.check_for_updates_on_startup
    )
}

fn validate_project_settings(
    settings: ProjectSettingsInput,
    previous_settings: &StoredProjectSettings,
) -> Result<StoredProjectSettings, String> {
    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;
    let incoming_official_gamelogos_dir = existing_optional_dir(settings.official_gamelogos_dir);
    let official_gamelogos_dir = if incoming_was_previous_auto_gamelogos_dir(
        &incoming_official_gamelogos_dir,
        previous_settings,
    ) {
        derive_official_gamelogos_dir(&mods_dir)
    } else {
        incoming_official_gamelogos_dir.or_else(|| derive_official_gamelogos_dir(&mods_dir))
    };

    Ok(StoredProjectSettings {
        disclaimer_accepted: settings.disclaimer_accepted.unwrap_or(false),
        mods_dir: Some(normalize_settings_path_string(
            mods_dir.to_string_lossy().into_owned(),
        )),
        official_gamelogos_dir: official_gamelogos_dir
            .map(|path| normalize_settings_path_string(path.to_string_lossy().into_owned())),
        keep_original_song_ini: settings.keep_original_song_ini.unwrap_or(true),
        check_for_updates_on_startup: settings.check_for_updates_on_startup.unwrap_or(true),
    })
}

fn incoming_was_previous_auto_gamelogos_dir(
    incoming_official_gamelogos_dir: &Option<PathBuf>,
    previous_settings: &StoredProjectSettings,
) -> bool {
    let Some(incoming_official_gamelogos_dir) = incoming_official_gamelogos_dir else {
        return false;
    };
    let Some(previous_official_gamelogos_dir) =
        existing_optional_dir(previous_settings.official_gamelogos_dir.clone())
    else {
        return false;
    };
    let Some(previous_mods_dir) = previous_settings.mods_dir.as_deref() else {
        return false;
    };
    let Some(previous_auto_gamelogos_dir) =
        derive_official_gamelogos_dir(Path::new(previous_mods_dir))
    else {
        return false;
    };

    incoming_official_gamelogos_dir == &previous_official_gamelogos_dir
        && incoming_official_gamelogos_dir == &previous_auto_gamelogos_dir
}

fn existing_optional_dir(path: Option<String>) -> Option<PathBuf> {
    let path = path?;

    if path.trim().is_empty() {
        return None;
    }

    let path = PathBuf::from(path);

    if !path.is_dir() {
        return None;
    }

    path.canonicalize().ok().map(normalize_settings_path)
}

fn derive_official_gamelogos_dir(mods_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(mods_dir);

    while let Some(path) = current {
        if path.file_name() == Some(OsStr::new("MODS")) {
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let candidate = parent.join("IMAGES").join("GAMELOGOS");

            if candidate.is_dir() {
                if let Ok(candidate) = candidate.canonicalize() {
                    return Some(normalize_settings_path(candidate));
                }
            }
        }

        current = path.parent();
    }

    None
}

fn required_existing_dir(path: Option<String>, label: &str) -> Result<PathBuf, String> {
    match path {
        Some(path) if !path.trim().is_empty() => existing_dir(path, label),
        _ => Err(format!("{label} is required.")),
    }
}

fn existing_dir(path: String, label: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    if !path.is_dir() {
        return Err(format!("{label} does not exist or is not a directory."));
    }

    path.canonicalize()
        .map(normalize_settings_path)
        .map_err(|err| format!("Failed to resolve {label}: {err}"))
}

fn unescape_ini_value(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();

    while let Some(char) = chars.next() {
        if char == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some(next) => {
                    result.push('\\');
                    result.push(next);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(char);
        }
    }

    result
}

#[cfg(windows)]
fn normalize_settings_path(path: PathBuf) -> PathBuf {
    PathBuf::from(normalize_settings_path_string(
        path.to_string_lossy().into_owned(),
    ))
}

#[cfg(not(windows))]
fn normalize_settings_path(path: PathBuf) -> PathBuf {
    path
}

#[cfg(windows)]
fn normalize_settings_path_string(path: String) -> String {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{path}");
    }

    path.strip_prefix(r"\\?\").unwrap_or(&path).to_string()
}

#[cfg(not(windows))]
fn normalize_settings_path_string(path: String) -> String {
    path
}

fn settings_error(err: io::Error) -> String {
    format!("Failed to read settings file: {err}")
}

fn parse_ini_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl Default for StoredProjectSettings {
    fn default() -> Self {
        Self {
            disclaimer_accepted: false,
            mods_dir: None,
            official_gamelogos_dir: None,
            keep_original_song_ini: true,
            check_for_updates_on_startup: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn third_party_licenses_include_new_rocker_ofl_text() {
        let licenses = third_party_licenses();

        assert!(licenses.contains("Copyright (c) 2011, Pablo Impallari"));
        assert!(licenses.contains("SIL OPEN FONT LICENSE Version 1.1"));
    }
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestProject {
        root: PathBuf,
        mods_dir: PathBuf,
    }

    impl TestProject {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "ghwtdeinitool-{name}-{}-{unique}",
                std::process::id()
            ));
            let mods_dir = root.join("MODS");

            fs::create_dir_all(&mods_dir).expect("mods dir should be created");

            Self { root, mods_dir }
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn test_scan_settings(project: &TestProject) -> ScanSettings {
        ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: None,
            keep_original_song_ini: true,
        }
    }

    fn test_scan_settings_without_song_ini_backup(project: &TestProject) -> ScanSettings {
        ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: None,
            keep_original_song_ini: false,
        }
    }

    #[test]
    fn categorize_songs_creates_sanitized_category_folders_and_updates_song_files() {
        let project = TestProject::new("categorize-songs");
        let settings = test_scan_settings_without_song_ini_backup(&project);
        let store = SongIniStore::default();
        let mut ordered_paths = Vec::new();
        write_test_file_contents(
            &project.mods_dir.join("IniToolCategories/Old/nested.txt"),
            "old category content",
        );

        for index in 0..201 {
            let relative_path = format!("Songs/{index:03}/song.ini");
            write_test_file_contents(
                &project.mods_dir.join(&relative_path),
                valid_song_ini(&format!("Song {index}"), &format!("checksum_{index}")).as_str(),
            );
            ordered_paths.push(relative_path);
        }

        scan_song_ini_files_paths(&settings, &store).expect("scan should succeed");
        let result = categorize_songs_paths(
            &settings,
            CategorizeSongsInput {
                ordered_song_paths: ordered_paths.clone(),
                included_song_paths: ordered_paths,
                maximum_song_cap: 200,
                category_names: vec!["01 Artist: A/B".to_string()],
            },
            &store,
        )
        .expect("categorization should succeed");

        assert_eq!(result.categorized, 200);
        assert_eq!(result.excluded, 1);
        let category_ini = project
            .mods_dir
            .join("IniToolCategories/Category_01_Artist_AB/category.ini");
        assert_eq!(
            fs::read_to_string(category_ini).expect("category should exist"),
            "[ModInfo]\nName=IniTool Category 01\nDescription=Songs categorized by 01 Artist: A/B.\nAuthor=GhwtDeIniTool\nVersion=1.0\n\n[CategoryInfo]\nName=01 Artist: A/B\nChecksum=IniToolCategory01\nLogo=gamelogo_initool01\n"
        );
        let category_folder = project
            .mods_dir
            .join("IniToolCategories/Category_01_Artist_AB");
        let img_xen = fs::read(category_folder.join("gamelogo_initool01.img.xen"))
            .expect("category logo should exist");
        assert!(img_xen.len() > IMG_XEN_HEADER_SIZE);
        assert_eq!(
            u32::from_be_bytes(img_xen[0..4].try_into().expect("IMG magic bytes")),
            0x0a28_1300
        );
        assert_eq!(
            u16::from_be_bytes(img_xen[8..10].try_into().expect("IMG width bytes")),
            CATEGORY_LOGO_SIZE as u16
        );
        assert_eq!(
            u16::from_be_bytes(img_xen[10..12].try_into().expect("IMG height bytes")),
            CATEGORY_LOGO_SIZE as u16
        );
        assert_eq!(img_xen[20], 1);
        assert_eq!(img_xen[21], 32);
        assert_eq!(img_xen[22], 0);
        assert_eq!(
            u32::from_be_bytes(img_xen[28..32].try_into().expect("IMG offset bytes")),
            IMG_XEN_HEADER_SIZE as u32
        );
        assert_eq!(
            u32::from_be_bytes(img_xen[32..36].try_into().expect("IMG size bytes")) as usize,
            img_xen.len() - IMG_XEN_HEADER_SIZE
        );
        assert_eq!(
            &img_xen[IMG_XEN_HEADER_SIZE..IMG_XEN_HEADER_SIZE + 8],
            PNG_SIGNATURE
        );
        let logo = image::load_from_memory(&img_xen[IMG_XEN_HEADER_SIZE..])
            .expect("embedded category logo PNG should decode")
            .to_rgba8();
        assert_eq!(logo.dimensions(), (CATEGORY_LOGO_SIZE, CATEGORY_LOGO_SIZE));
        assert!(logo.pixels().any(|pixel| pixel[3] == 0));
        assert!(logo
            .pixels()
            .any(|pixel| *pixel == Rgba([255, 255, 255, 255])));
        assert!(logo.pixels().any(|pixel| *pixel == Rgba([0, 0, 0, 255])));
        assert!(!category_folder.join("gamelogo_initool01.png").exists());
        assert!(
            fs::read_to_string(project.mods_dir.join("Songs/000/song.ini"))
                .expect("categorized song should exist")
                .contains("GameCategory=IniToolCategory01")
        );
        assert!(project
            .mods_dir
            .join("Songs/200/song.excluded.ini")
            .is_file());
        assert!(!project
            .mods_dir
            .join("IniToolCategories/Old/nested.txt")
            .exists());
    }

    #[test]
    fn categorize_songs_rejects_caps_outside_two_digit_category_limit() {
        let project = TestProject::new("categorize-cap-limit");
        let settings = test_scan_settings(&project);
        let store = SongIniStore::default();

        for maximum_song_cap in [0, 19_801] {
            let error = match categorize_songs_paths(
                &settings,
                CategorizeSongsInput {
                    ordered_song_paths: Vec::new(),
                    included_song_paths: Vec::new(),
                    maximum_song_cap,
                    category_names: Vec::new(),
                },
                &store,
            ) {
                Ok(_) => panic!("out-of-range cap should be rejected"),
                Err(error) => error,
            };
            assert_eq!(error, "Enter a whole number from 1 through 19800.");
        }
    }

    #[test]
    fn settings_default_to_keeping_original_song_ini() {
        let settings = read_project_settings_from_ini("[project]\nmods_dir=/tmp/MODS\n");

        assert!(!settings.disclaimer_accepted);
        assert_eq!(settings.mods_dir.as_deref(), Some("/tmp/MODS"));
        assert_eq!(settings.official_gamelogos_dir, None);
        assert!(settings.keep_original_song_ini);
        assert!(settings.check_for_updates_on_startup);
    }

    #[test]
    fn settings_parse_official_gamelogos_dir() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=/tmp/MODS\nofficial_gamelogos_dir=/tmp/IMAGES/GAMELOGOS\n",
        );

        assert_eq!(
            settings.official_gamelogos_dir.as_deref(),
            Some("/tmp/IMAGES/GAMELOGOS")
        );
    }

    #[test]
    fn settings_parse_keep_original_song_ini() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=/tmp/MODS\nkeep_original_song_ini=false\n",
        );

        assert!(!settings.keep_original_song_ini);
    }

    #[test]
    fn settings_parse_check_for_updates_on_startup() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=/tmp/MODS\ncheck_for_updates_on_startup=false\n",
        );

        assert!(!settings.check_for_updates_on_startup);
    }

    #[test]
    fn settings_parse_disclaimer_accepted() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=/tmp/MODS\ndisclaimer_accepted=true\n",
        );

        assert!(settings.disclaimer_accepted);
    }

    #[test]
    fn settings_writer_includes_disclaimer_accepted() {
        let settings = StoredProjectSettings {
            disclaimer_accepted: true,
            mods_dir: Some("/tmp/MODS".to_string()),
            official_gamelogos_dir: Some("/tmp/IMAGES/GAMELOGOS".to_string()),
            keep_original_song_ini: false,
            check_for_updates_on_startup: false,
        };

        assert_eq!(
            write_project_settings_to_ini(&settings),
            "[project]\ndisclaimer_accepted=true\nmods_dir=/tmp/MODS\nofficial_gamelogos_dir=/tmp/IMAGES/GAMELOGOS\nkeep_original_song_ini=false\ncheck_for_updates_on_startup=false\n"
        );
    }

    #[test]
    fn settings_writer_keeps_windows_backslashes_readable() {
        let settings = StoredProjectSettings {
            disclaimer_accepted: true,
            mods_dir: Some("C:\\Games\\GHWT\\DATA\\MODS".to_string()),
            official_gamelogos_dir: Some("C:\\Games\\GHWT\\DATA\\IMAGES\\GAMELOGOS".to_string()),
            keep_original_song_ini: true,
            check_for_updates_on_startup: true,
        };

        assert_eq!(
            write_project_settings_to_ini(&settings),
            "[project]\ndisclaimer_accepted=true\nmods_dir=C:\\Games\\GHWT\\DATA\\MODS\nofficial_gamelogos_dir=C:\\Games\\GHWT\\DATA\\IMAGES\\GAMELOGOS\nkeep_original_song_ini=true\ncheck_for_updates_on_startup=true\n"
        );
    }

    #[test]
    fn settings_reader_decodes_legacy_escaped_backslashes() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=C:\\\\Games\\\\GHWT\\\\DATA\\\\MODS\n",
        );

        assert_eq!(
            settings.mods_dir.as_deref(),
            Some("C:\\Games\\GHWT\\DATA\\MODS")
        );
    }

    #[test]
    fn settings_reader_keeps_single_backslashes_literal() {
        let settings = read_project_settings_from_ini("[project]\nmods_dir=C:\\new\\MODS\n");

        assert_eq!(settings.mods_dir.as_deref(), Some("C:\\new\\MODS"));
    }

    #[cfg(windows)]
    #[test]
    fn settings_normalize_windows_extended_paths() {
        assert_eq!(
            normalize_settings_path_string(r"\\?\C:\Games\GHWT".to_string()),
            r"C:\Games\GHWT"
        );
        assert_eq!(
            normalize_settings_path_string(r"\\?\UNC\server\share\MODS".to_string()),
            r"\\server\share\MODS"
        );
        assert_eq!(
            normalize_settings_path_string(r"C:\Games\GHWT".to_string()),
            r"C:\Games\GHWT"
        );
    }

    #[cfg(windows)]
    #[test]
    fn checked_mods_paths_accept_normalized_windows_roots() {
        let project = TestProject::new("normalized-windows-mods-root");
        let song_dir = project.mods_dir.join("BH").join("A Million Ways");
        let song_ini = song_dir.join("song.ini");
        write_test_file_contents(&song_ini, &valid_song_ini("A Million Ways", "millionways"));
        let mods_dir = normalize_settings_path(
            project
                .mods_dir
                .canonicalize()
                .expect("MODS directory should resolve"),
        );

        assert_eq!(
            checked_mods_relative_path(&mods_dir, "BH/A Million Ways/song.ini")
                .expect("song inside MODS should be accepted"),
            mods_dir.join("BH").join("A Million Ways").join("song.ini")
        );
        assert_eq!(
            checked_mods_relative_path_allow_missing(&mods_dir, "BH/A Million Ways/new.ini")
                .expect("new file under MODS should be accepted"),
            mods_dir.join("BH").join("A Million Ways").join("new.ini")
        );
    }

    #[test]
    fn settings_derives_official_gamelogos_dir_from_mods_dir() {
        let project = TestProject::new("derive-gamelogos");
        let mods_dir = project.root.join("MODS").join("BH");
        let gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        fs::create_dir_all(&mods_dir).expect("nested mods dir should be created");
        fs::create_dir_all(&gamelogos_dir).expect("gamelogos dir should be created");

        assert_eq!(
            derive_official_gamelogos_dir(&mods_dir).as_deref(),
            Some(gamelogos_dir.canonicalize().unwrap().as_path())
        );
    }

    #[test]
    fn settings_derives_official_gamelogos_dir_from_next_mods_match_when_nearest_is_missing() {
        let project = TestProject::new("derive-gamelogos-fallback");
        let mods_dir = project
            .root
            .join("MODS")
            .join("foo")
            .join("MODS")
            .join("BH");
        let gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        fs::create_dir_all(&mods_dir).expect("nested mods dir should be created");
        fs::create_dir_all(&gamelogos_dir).expect("gamelogos dir should be created");

        assert_eq!(
            derive_official_gamelogos_dir(&mods_dir).as_deref(),
            Some(gamelogos_dir.canonicalize().unwrap().as_path())
        );
    }

    #[test]
    fn settings_derives_official_gamelogos_dir_from_nearest_mods_match_first() {
        let project = TestProject::new("derive-gamelogos-nearest");
        let mods_dir = project
            .root
            .join("MODS")
            .join("foo")
            .join("MODS")
            .join("BH");
        let nearest_gamelogos_dir = project
            .root
            .join("MODS")
            .join("foo")
            .join("IMAGES")
            .join("GAMELOGOS");
        let farther_gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        fs::create_dir_all(&mods_dir).expect("nested mods dir should be created");
        fs::create_dir_all(&nearest_gamelogos_dir)
            .expect("nearest gamelogos dir should be created");
        fs::create_dir_all(&farther_gamelogos_dir)
            .expect("farther gamelogos dir should be created");

        assert_eq!(
            derive_official_gamelogos_dir(&mods_dir).as_deref(),
            Some(nearest_gamelogos_dir.canonicalize().unwrap().as_path())
        );
    }

    #[test]
    fn settings_does_not_derive_official_gamelogos_dir_from_inexact_mods_match() {
        let project = TestProject::new("derive-gamelogos-inexact");
        let lowercase_mods_dir = project.root.join("mods").join("BH");
        let containing_mods_dir = project.root.join("MODS_extra").join("BH");
        let gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        fs::create_dir_all(&lowercase_mods_dir).expect("lowercase mods dir should be created");
        fs::create_dir_all(&containing_mods_dir).expect("containing mods dir should be created");
        fs::create_dir_all(&gamelogos_dir).expect("gamelogos dir should be created");

        assert_eq!(derive_official_gamelogos_dir(&lowercase_mods_dir), None);
        assert_eq!(derive_official_gamelogos_dir(&containing_mods_dir), None);
    }

    #[test]
    fn settings_preserves_existing_official_gamelogos_dir() {
        let project = TestProject::new("preserve-manual-gamelogos");
        let mods_dir = project.root.join("MODS").join("BH");
        let manual_gamelogos_dir = project.root.join("ManualGameLogos");
        let derived_gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        fs::create_dir_all(&mods_dir).expect("nested mods dir should be created");
        fs::create_dir_all(&manual_gamelogos_dir).expect("manual gamelogos dir should be created");
        fs::create_dir_all(&derived_gamelogos_dir)
            .expect("derived gamelogos dir should be created");

        let settings = validate_project_settings(
            ProjectSettingsInput {
                disclaimer_accepted: Some(true),
                mods_dir: Some(mods_dir.to_string_lossy().into_owned()),
                official_gamelogos_dir: Some(manual_gamelogos_dir.to_string_lossy().into_owned()),
                keep_original_song_ini: Some(false),
                check_for_updates_on_startup: None,
            },
            &StoredProjectSettings::default(),
        )
        .expect("settings should be valid");

        assert_eq!(
            settings.official_gamelogos_dir.as_deref(),
            Some(
                manual_gamelogos_dir
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert!(settings.disclaimer_accepted);
        assert!(!settings.keep_original_song_ini);
    }

    #[test]
    fn settings_replaces_previous_auto_official_gamelogos_dir_when_mods_changes() {
        let project = TestProject::new("replace-auto-gamelogos");
        let previous_mods_dir = project.root.join("MODS").join("BH");
        let previous_auto_gamelogos_dir = project.root.join("IMAGES").join("GAMELOGOS");
        let new_auto_gamelogos_dir = project
            .root
            .join("Official")
            .join("IMAGES")
            .join("GAMELOGOS");
        fs::create_dir_all(&previous_mods_dir).expect("previous mods dir should be created");
        fs::create_dir_all(&previous_auto_gamelogos_dir)
            .expect("previous gamelogos dir should be created");
        let new_mods_dir = project.root.join("Official").join("MODS").join("GH");
        fs::create_dir_all(&new_mods_dir).expect("new mods dir should be created");
        fs::create_dir_all(&new_auto_gamelogos_dir).expect("new gamelogos dir should be created");

        let previous_settings = StoredProjectSettings {
            disclaimer_accepted: true,
            mods_dir: Some(previous_mods_dir.to_string_lossy().into_owned()),
            official_gamelogos_dir: Some(
                previous_auto_gamelogos_dir.to_string_lossy().into_owned(),
            ),
            keep_original_song_ini: true,
            check_for_updates_on_startup: true,
        };
        let settings = validate_project_settings(
            ProjectSettingsInput {
                disclaimer_accepted: Some(true),
                mods_dir: Some(new_mods_dir.to_string_lossy().into_owned()),
                official_gamelogos_dir: Some(
                    previous_auto_gamelogos_dir.to_string_lossy().into_owned(),
                ),
                keep_original_song_ini: None,
                check_for_updates_on_startup: None,
            },
            &previous_settings,
        )
        .expect("settings should be valid");

        assert_eq!(
            settings.official_gamelogos_dir.as_deref(),
            Some(
                new_auto_gamelogos_dir
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
    }

    #[test]
    fn settings_allows_missing_official_gamelogos_dir() {
        let project = TestProject::new("missing-gamelogos");
        let settings = validate_project_settings(
            ProjectSettingsInput {
                disclaimer_accepted: None,
                mods_dir: Some(project.mods_dir.to_string_lossy().into_owned()),
                official_gamelogos_dir: None,
                keep_original_song_ini: None,
                check_for_updates_on_startup: None,
            },
            &StoredProjectSettings::default(),
        )
        .expect("settings should be valid without gamelogos");

        assert_eq!(settings.official_gamelogos_dir, None);
    }

    #[test]
    fn instrument_column_uses_highest_available_level() {
        let column = instrument_column("Guitar", true, true, false, true, &[]);

        assert_eq!(column.value, "Expert");
        assert!(column.easy);
        assert!(column.medium);
        assert!(!column.hard);
        assert!(column.expert);
        assert!(column.tooltip.contains("Easy Yes"));
        assert!(column.tooltip.contains("Hard No"));
    }

    #[test]
    fn instrument_column_only_displays_error_when_no_level_is_available() {
        let errors = vec!["Pak could not be parsed.".to_string()];
        let playable_column = instrument_column("Bass", false, true, false, false, &errors);
        let empty_column = instrument_column("Drums", false, false, false, false, &errors);

        assert_eq!(playable_column.value, "Medium");
        assert_eq!(empty_column.value, "Error");
        assert_eq!(empty_column.tooltip, "Pak could not be parsed.");
    }

    #[test]
    fn vocals_column_maps_supported_vocals_to_expert() {
        let supported = vocals_column(true, &[]);
        let unsupported = vocals_column(false, &[]);
        let error = vocals_column(false, &["Missing vocals data.".to_string()]);

        assert_eq!(supported.value, "Expert");
        assert!(supported.expert);
        assert_eq!(unsupported.value, "No");
        assert_eq!(error.value, "Error");
        assert_eq!(error.tooltip, "Missing vocals data.");
    }

    #[test]
    fn song_pak_path_matches_content_and_checksum_case_insensitively() {
        let project = TestProject::new("song-pak-case-insensitive");
        let song_dir = project.mods_dir.join("Artist - Song");
        let content_dir = song_dir.join("content");
        let song_ini_path = song_dir.join("song.ini");
        fs::create_dir_all(&content_dir).expect("content dir should be created");
        let pak_path = content_dir.join("aSAMPLE_song.PAK.XEN");
        write_test_file_contents(&song_ini_path, valid_song_ini("Sample", "sample").as_str());
        write_test_file(&pak_path);

        let identity = current_pak_identity(&song_ini_path, "sample");

        assert_eq!(identity.status, "present");
        assert_eq!(identity.relative_path, "content/aSAMPLE_song.PAK.XEN");
        assert_eq!(identity.size, Some(4));
    }

    #[test]
    fn song_scan_returns_unknown_instruments_when_sidecar_is_missing() {
        let project = TestProject::new("instrument-sidecar-missing");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Unknown", "unknown_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should succeed");

        assert_eq!(result.songs[0].instruments.guitar.value, "Unknown");
        assert_eq!(result.songs[0].instruments.vocals.value, "Unknown");
    }

    #[test]
    fn song_scan_reads_fresh_instrument_sidecar() {
        let project = TestProject::new("instrument-sidecar-fresh");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Cached", "cached_checksum").as_str(),
        );
        let identity = current_pak_identity(&song_ini_path, "cached_checksum");
        let instruments = ScannedSongInstruments {
            guitar: instrument_column("Guitar", true, true, false, true, &[]),
            bass: empty_column("Bass", &[]),
            drums: empty_column("Drums", &[]),
            vocals: vocals_column(true, &[]),
            coop_guitar: empty_column("CoopGuitar", &[]),
            coop_bass: empty_column("CoopBass", &[]),
        };
        write_instrument_sidecar(&song_ini_path, "cached_checksum", &identity, &instruments)
            .expect("sidecar should write");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should succeed");

        assert_eq!(result.songs[0].instruments.guitar.value, "Expert");
        assert_eq!(result.songs[0].instruments.vocals.value, "Expert");
    }

    #[test]
    fn song_scan_ignores_stale_instrument_sidecar() {
        let project = TestProject::new("instrument-sidecar-stale");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Stale", "current_checksum").as_str(),
        );
        let identity = current_pak_identity(&song_ini_path, "old_checksum");
        let instruments = ScannedSongInstruments {
            guitar: instrument_column("Guitar", false, false, false, true, &[]),
            bass: empty_column("Bass", &[]),
            drums: empty_column("Drums", &[]),
            vocals: empty_column("Vocals", &[]),
            coop_guitar: empty_column("CoopGuitar", &[]),
            coop_bass: empty_column("CoopBass", &[]),
        };
        write_instrument_sidecar(&song_ini_path, "old_checksum", &identity, &instruments)
            .expect("sidecar should write");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should succeed");

        assert_eq!(result.songs[0].instruments.guitar.value, "Unknown");
    }

    #[test]
    fn instrument_analyze_modes_select_expected_songs() {
        let project = TestProject::new("instrument-analyze-modes");
        fs::create_dir_all(project.mods_dir.join("missing")).expect("missing dir should exist");
        fs::create_dir_all(project.mods_dir.join("error")).expect("error dir should exist");
        fs::create_dir_all(project.mods_dir.join("cached")).expect("cached dir should exist");
        write_test_file_contents(
            &project.mods_dir.join("missing").join("song.ini"),
            valid_song_ini("Missing", "missing_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("error").join("song.ini"),
            valid_song_ini("Error", "error_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("cached").join("song.ini"),
            valid_song_ini("Cached", "cached_checksum").as_str(),
        );
        let error_song_ini = project.mods_dir.join("error").join("song.ini");
        let cached_song_ini = project.mods_dir.join("cached").join("song.ini");
        let error_identity = current_pak_identity(&error_song_ini, "error_checksum");
        let error_instruments = error_instruments(vec!["Cached analyzer error.".to_string()]);
        write_instrument_sidecar(
            &error_song_ini,
            "error_checksum",
            &error_identity,
            &error_instruments,
        )
        .expect("error sidecar should write");
        let cached_identity = current_pak_identity(&cached_song_ini, "cached_checksum");
        let cached_instruments = ScannedSongInstruments {
            guitar: instrument_column("Guitar", false, false, false, true, &[]),
            bass: empty_column("Bass", &[]),
            drums: empty_column("Drums", &[]),
            vocals: empty_column("Vocals", &[]),
            coop_guitar: empty_column("CoopGuitar", &[]),
            coop_bass: empty_column("CoopBass", &[]),
        };
        write_instrument_sidecar(
            &cached_song_ini,
            "cached_checksum",
            &cached_identity,
            &cached_instruments,
        )
        .expect("cached sidecar should write");
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let errors_result = analyze_scanned_song_instruments_paths(
            &test_scan_settings(&project),
            InstrumentAnalyzeMode::Errors,
            &store,
            |_| {},
        )
        .expect("error analysis should succeed");
        assert_eq!(errors_result.analyzed, 1);
        assert_eq!(errors_result.skipped, 2);

        let missing_result = analyze_scanned_song_instruments_paths(
            &test_scan_settings(&project),
            InstrumentAnalyzeMode::Missing,
            &store,
            |_| {},
        )
        .expect("missing analysis should succeed");
        assert_eq!(missing_result.analyzed, 1);
        assert_eq!(missing_result.skipped, 2);

        let all_result = analyze_scanned_song_instruments_paths(
            &test_scan_settings(&project),
            InstrumentAnalyzeMode::All,
            &store,
            |_| {},
        )
        .expect("all analysis should succeed");
        assert_eq!(all_result.analyzed, 3);
        assert_eq!(all_result.skipped, 0);
    }

    #[test]
    fn instrument_analysis_writes_sidecar() {
        let project = TestProject::new("instrument-analysis-writes");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Write", "write_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = analyze_scanned_song_instruments_paths(
            &test_scan_settings(&project),
            InstrumentAnalyzeMode::Missing,
            &store,
            |_| {},
        )
        .expect("analysis should succeed");

        assert_eq!(result.analyzed, 1);
        assert!(instrument_sidecar_path(&song_ini_path).is_file());
    }

    #[test]
    fn instrument_analysis_emits_initial_and_final_progress() {
        let project = TestProject::new("instrument-analysis-progress");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Progress", "progress_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");
        let mut progress_events = Vec::new();

        analyze_scanned_song_instruments_paths(
            &test_scan_settings(&project),
            InstrumentAnalyzeMode::Missing,
            &store,
            |progress| progress_events.push(progress),
        )
        .expect("analysis should succeed");

        let first = progress_events
            .first()
            .expect("initial progress should be emitted");
        assert_eq!(first.current, 0);
        assert_eq!(first.total, 1);
        let last = progress_events
            .last()
            .expect("final progress should be emitted");
        assert_eq!(last.current, 1);
        assert_eq!(last.total, 1);
    }

    #[test]
    fn instrument_sidecar_is_not_scanned_as_active_song_ini() {
        let project = TestProject::new("instrument-sidecar-not-active");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join(INSTRUMENT_SIDECAR_FILE_NAME),
            "[Cache]\nChecksum=active_checksum\n",
        );

        let song_ini_paths =
            find_song_ini_files(&project.mods_dir).expect("song files should scan");

        assert_eq!(song_ini_paths, vec![project.mods_dir.join("song.ini")]);
    }

    #[test]
    fn song_scan_emits_reading_progress() {
        let project = TestProject::new("song-scan-reading-progress");
        fs::create_dir_all(project.mods_dir.join("first")).expect("first dir should exist");
        fs::create_dir_all(project.mods_dir.join("second")).expect("second dir should exist");
        write_test_file_contents(
            &project.mods_dir.join("first").join("song.ini"),
            valid_song_ini("First", "first_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("second").join("song.ini"),
            valid_song_ini("Second", "second_checksum").as_str(),
        );
        let store = SongIniStore::default();
        let mut progress = Vec::new();

        scan_song_ini_files_paths_with_progress(&test_scan_settings(&project), &store, |event| {
            progress.push(event);
        })
        .expect("scan should succeed");

        let reading_events = progress
            .iter()
            .filter(|event| event.phase == "readingSongs")
            .collect::<Vec<_>>();
        assert_eq!(reading_events.len(), 1);
        assert_eq!(reading_events[0].current, 1);
        assert_eq!(reading_events[0].total, 2);
    }

    #[test]
    fn song_scan_emits_content_progress() {
        let project = TestProject::new("song-scan-content-progress");
        fs::create_dir_all(project.mods_dir.join("first")).expect("first dir should exist");
        fs::create_dir_all(project.mods_dir.join("second")).expect("second dir should exist");
        write_test_file_contents(
            &project.mods_dir.join("first").join("song.ini"),
            valid_song_ini("First", "first_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("second").join("song.ini"),
            valid_song_ini("Second", "second_checksum").as_str(),
        );
        let store = SongIniStore::default();
        let mut progress = Vec::new();

        scan_song_ini_files_paths_with_progress(&test_scan_settings(&project), &store, |event| {
            progress.push(event);
        })
        .expect("scan should succeed");

        let content_events = progress
            .iter()
            .filter(|event| event.phase == "checkingContent")
            .collect::<Vec<_>>();
        assert_eq!(content_events.len(), 1);
        assert_eq!(content_events[0].current, 1);
        assert_eq!(content_events[0].total, 2);
    }

    #[test]
    fn song_scan_progress_emitter_throttles_items_but_emits_phase_changes_and_completion() {
        let mut events = Vec::new();

        {
            let mut emitter = SongScanProgressEmitter::new(|event| events.push(event));
            emitter.emit(song_scan_progress("findingSongs", 0, 0, ""), true);
            emitter.emit(
                song_scan_progress("readingSongs", 1, 3, "first/song.ini"),
                false,
            );
            emitter.emit(
                song_scan_progress("readingSongs", 2, 3, "second/song.ini"),
                false,
            );
            emitter.emit(
                song_scan_progress("checkingContent", 1, 3, "first/song.ini"),
                false,
            );
            emitter.emit(song_scan_progress("finishing", 0, 0, ""), true);
        }

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].phase, "findingSongs");
        assert_eq!(events[1].phase, "readingSongs");
        assert_eq!(events[1].current, 1);
        assert_eq!(events[2].phase, "checkingContent");
        assert_eq!(events[2].current, 1);
        assert_eq!(events[3].phase, "finishing");
    }

    #[test]
    fn delete_preview_keeps_files_by_case_insensitive_filename_patterns() {
        let project = TestProject::new("keep-patterns");
        write_test_file(&project.mods_dir.join("SONG.INI"));
        write_test_file(&project.mods_dir.join("Track.FSB.XEN"));
        write_test_file(&project.mods_dir.join("Lead_SONG.PAK.XEN"));
        write_test_file(&project.mods_dir.join("notes.txt"));

        let preview = preview_keep_only_files_delete_paths(
            &project.mods_dir,
            " song.ini, *_song.pak.xen, *.fsb.xen ,, ",
        )
        .expect("preview should succeed");

        assert_eq!(preview.files_to_delete, vec!["notes.txt".to_string()]);
        assert!(preview.errors.is_empty());
    }

    #[test]
    fn blank_delete_preview_keeps_all_files() {
        let project = TestProject::new("blank-keep-patterns");
        write_test_file(&project.mods_dir.join("notes.txt"));

        let preview = preview_keep_only_files_delete_paths(&project.mods_dir, " ,  , ")
            .expect("preview should succeed");

        assert!(preview.files_to_delete.is_empty());
        assert!(project.mods_dir.join("notes.txt").exists());
    }

    #[test]
    fn delete_confirm_removes_confirmed_relative_files_only() {
        let project = TestProject::new("delete-confirmed");
        write_test_file(&project.mods_dir.join("notes.txt"));
        write_test_file(&project.mods_dir.join("song.ini"));

        let result = delete_keep_only_files_paths(&project.mods_dir, &["notes.txt".to_string()])
            .expect("delete should complete");

        assert_eq!(result.files_deleted, 1);
        assert!(result.errors.is_empty());
        assert!(!project.mods_dir.join("notes.txt").exists());
        assert!(project.mods_dir.join("song.ini").exists());
    }

    #[test]
    fn delete_confirm_rejects_paths_outside_mods() {
        let project = TestProject::new("reject-delete-outside");
        let outside_file = project.root.join("outside.txt");
        write_test_file(&outside_file);

        let result =
            delete_keep_only_files_paths(&project.mods_dir, &["../outside.txt".to_string()])
                .expect("delete should complete with errors");

        assert_eq!(result.files_deleted, 0);
        assert_eq!(
            result.errors,
            vec!["Rejected unsafe MODS-relative path ../outside.txt.".to_string()]
        );
        assert!(outside_file.exists());
    }

    #[test]
    fn song_scan_finds_recursive_song_ini_files_case_insensitively() {
        let project = TestProject::new("song-scan-recursive");
        let song_dir = project.mods_dir.join("Artist").join("Song");
        fs::create_dir_all(&song_dir).expect("song folder should be created");
        write_test_file_contents(
            &project.mods_dir.join("SONG.INI"),
            "[SongInfo]\nTitle=Root\n",
        );
        write_test_file_contents(&song_dir.join("song.ini"), "[SongInfo]\nTitle=Nested\n");

        let song_ini_paths =
            find_song_ini_files(&project.mods_dir).expect("song.ini discovery should work");
        let relative_paths = song_ini_paths
            .iter()
            .map(|path| mods_relative_path(&project.mods_dir, path).expect("relative path"))
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec!["Artist/Song/song.ini".to_string(), "SONG.INI".to_string()]
        );
    }

    #[test]
    fn song_scan_stores_valid_song_ini_files() {
        let project = TestProject::new("song-scan-valid");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "valid_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert!(result.faulty_files.is_empty());
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].relative_path, "song.ini");
        assert_eq!(stored_songs[0].sections[0].name.as_deref(), Some("ModInfo"));
        assert_eq!(
            stored_songs[0].sections[1].name.as_deref(),
            Some("SongInfo")
        );
        assert_eq!(stored_songs[0].sections[1].entries[0].key, "Checksum");
        assert_eq!(
            stored_songs[0].sections[1].entries[0].value,
            "valid_checksum"
        );
        assert_eq!(stored_songs[0].sections[1].entries[1].key, "Title");
        assert_eq!(stored_songs[0].sections[1].entries[1].value, "Valid");
    }

    #[test]
    fn song_scan_returns_display_rows_from_song_info() {
        let project = TestProject::new("song-scan-display-rows");
        let song_dir = project.mods_dir.join("Artist").join("Song");
        fs::create_dir_all(&song_dir).expect("song folder should be created");
        write_test_file_contents(
            &song_dir.join("song.ini"),
            "[ModInfo]\nName=Display Row\n\n[SongInfo]\nChecksum=display_checksum\nArtist=The Artist\nTitle=The Title\nYear=1984\nGenre=Rock\nGameIcon=ghwt\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs.len(), 1);
        assert_eq!(result.songs[0].relative_path, "Artist/Song/song.ini");
        assert_eq!(
            result.songs[0].folder_absolute_path,
            song_dir.display().to_string()
        );
        assert_eq!(result.songs[0].artist, "The Artist");
        assert_eq!(result.songs[0].title, "The Title");
        assert_eq!(result.songs[0].year, "1984");
        assert_eq!(result.songs[0].genre, "Rock");
        assert_eq!(result.songs[0].game_icon, "ghwt");
        assert!(!result.songs[0].has_original_song_ini);
    }

    #[test]
    fn song_scan_marks_rows_with_original_backup() {
        let project = TestProject::new("song-scan-original-backup");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.original.ini"),
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs.len(), 1);
        assert!(result.songs[0].has_original_song_ini);
    }

    #[test]
    fn song_scan_ignores_faulty_original_backup_for_restore_state() {
        let project = TestProject::new("song-scan-faulty-original-backup");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.original.faulty.ini"),
            "[SongInfo\nTitle=Broken\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs.len(), 1);
        assert!(!result.songs[0].has_original_song_ini);
    }

    #[test]
    fn song_scan_returns_empty_display_values_for_missing_optional_keys() {
        let project = TestProject::new("song-scan-display-empty");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[ModInfo]\nName=Minimal\n\n[SongInfo]\nChecksum=minimal_checksum\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 1);
        assert_eq!(result.songs.len(), 1);
        assert_eq!(result.songs[0].artist, "");
        assert_eq!(result.songs[0].title, "");
        assert_eq!(result.songs[0].year, "");
        assert_eq!(result.songs[0].genre, "");
        assert_eq!(result.songs[0].game_icon, "");
    }

    #[test]
    fn song_scan_accepts_utf8_bom_before_modinfo() {
        let project = TestProject::new("song-scan-bom");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "\u{feff}[ModInfo]\r\nName=With Bom\r\n\r\n[SongInfo]\r\nChecksum=with_bom\r\nTitle=With Bom\r\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert!(result.faulty_files.is_empty());
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].sections[0].name.as_deref(), Some("ModInfo"));
    }

    #[test]
    fn song_scan_auto_corrects_known_keys_with_wrong_case() {
        let project = TestProject::new("song-scan-key-case");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_contents =
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nartist=Motorhead\n";
        write_test_file_contents(&song_ini_path, original_contents);
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert!(result.faulty_files.is_empty());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nArtist=Motorhead\n"
        );
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("song.original.ini should read"),
            original_contents
        );
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].sections[1].entries[1].key, "Artist");
    }

    #[test]
    fn song_scan_allows_unknown_extra_keys() {
        let project = TestProject::new("song-scan-extra-keys");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[ModInfo]\nName=Extra Keys\nCustomModInfo=value\n\n[SongInfo]\nChecksum=extra_keys\nCustomSongInfo=value\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert!(result.faulty_files.is_empty());
        assert_eq!(stored_songs.len(), 1);
    }

    #[test]
    fn song_scan_returns_faulty_file_contents_and_parse_errors() {
        let project = TestProject::new("song-scan-invalid");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[SongInfo\nTitle=Broken\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 0);
        assert_eq!(result.faulty_files.len(), 1);
        assert_eq!(result.faulty_files[0].relative_path, "song.ini");
        assert_eq!(result.faulty_files[0].contents, "[SongInfo\nTitle=Broken\n");
        assert!(result.faulty_files[0].error.contains("song.ini"));
        assert!(stored_songs.is_empty());
    }

    #[test]
    fn song_scan_rejects_duplicate_keys_in_same_section() {
        let project = TestProject::new("song-scan-duplicates");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[ModInfo]\nName=Duplicate\n\n[SongInfo]\nChecksum=duplicate\nArtist=OK Go\nArtist=OK Go\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 0);
        assert_eq!(result.faulty_files.len(), 1);
        assert!(result.faulty_files[0]
            .error
            .contains("duplicate key `artist`"));
        assert!(result.faulty_files[0].error.contains("[SongInfo]"));
        assert!(stored_songs.is_empty());
    }

    #[test]
    fn song_scan_requires_case_sensitive_modinfo_and_songinfo_sections() {
        let project = TestProject::new("song-scan-required-sections");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[modinfo]\nName=Wrong Case\n\n[SongInfo]\nChecksum=wrongcase\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 0);
        assert_eq!(result.faulty_files.len(), 1);
        assert!(result.faulty_files[0]
            .error
            .contains("missing required [ModInfo] section"));
    }

    #[test]
    fn song_scan_requires_non_empty_checksum_in_songinfo() {
        let project = TestProject::new("song-scan-required-checksum");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "[ModInfo]\nName=Missing Checksum\n\n[SongInfo]\nChecksum=   \n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 0);
        assert_eq!(result.faulty_files.len(), 1);
        assert!(result.faulty_files[0]
            .error
            .contains("Checksum entry in [SongInfo] must not be empty"));
    }

    #[test]
    fn song_scan_groups_duplicate_checksums_from_songinfo() {
        let project = TestProject::new("song-scan-duplicate-checksums");
        let first_dir = project.mods_dir.join("First");
        let second_dir = project.mods_dir.join("Second");
        fs::create_dir_all(&first_dir).expect("first song folder should be created");
        fs::create_dir_all(&second_dir).expect("second song folder should be created");
        write_test_file_contents(
            &first_dir.join("song.ini"),
            valid_song_ini("First", "Rush2112P1").as_str(),
        );
        write_test_file_contents(
            &second_dir.join("song.ini"),
            valid_song_ini("Second", "  Rush2112P1  ").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 2);
        assert_eq!(result.duplicate_checksum_groups.len(), 1);
        assert_eq!(result.duplicate_checksum_groups[0].checksum, "Rush2112P1");
        assert_eq!(
            result.duplicate_checksum_groups[0].relative_paths,
            vec!["First/song.ini".to_string(), "Second/song.ini".to_string()]
        );
    }

    #[test]
    fn song_scan_ignores_duplicate_looking_checksum_outside_songinfo() {
        let project = TestProject::new("song-scan-checksum-section");
        let first_dir = project.mods_dir.join("First");
        let second_dir = project.mods_dir.join("Second");
        fs::create_dir_all(&first_dir).expect("first song folder should be created");
        fs::create_dir_all(&second_dir).expect("second song folder should be created");
        write_test_file_contents(
            &first_dir.join("song.ini"),
            "[ModInfo]\nName=First\nChecksum=NotSongInfo\n\n[SongInfo]\nChecksum=FirstChecksum\nTitle=First\n",
        );
        write_test_file_contents(
            &second_dir.join("song.ini"),
            "[ModInfo]\nName=Second\nChecksum=NotSongInfo\n\n[SongInfo]\nChecksum=SecondChecksum\nTitle=Second\n",
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 2);
        assert!(result.duplicate_checksum_groups.is_empty());
    }

    #[test]
    fn song_scan_reports_no_duplicate_groups_for_unique_checksums() {
        let project = TestProject::new("song-scan-unique-checksums");
        let first_dir = project.mods_dir.join("First");
        let second_dir = project.mods_dir.join("Second");
        fs::create_dir_all(&first_dir).expect("first song folder should be created");
        fs::create_dir_all(&second_dir).expect("second song folder should be created");
        write_test_file_contents(
            &first_dir.join("song.ini"),
            valid_song_ini("First", "first_checksum").as_str(),
        );
        write_test_file_contents(
            &second_dir.join("song.ini"),
            valid_song_ini("Second", "second_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_parsed, 2);
        assert!(result.duplicate_checksum_groups.is_empty());
    }

    #[test]
    fn song_scan_accepts_valid_content_layout() {
        let project = TestProject::new("song-scan-valid-content");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "valid_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "valid_checksum");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert!(result.content_file_issues.is_empty());
    }

    #[test]
    fn song_scan_accepts_content_and_music_folder_casing_without_renaming() {
        let project = TestProject::new("song-scan-content-folder-case");
        let content_dir = project.mods_dir.join("content");
        let music_dir = content_dir.join("music");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "case_checksum").as_str(),
        );
        fs::create_dir_all(&music_dir).expect("music dir should be created");
        write_test_file(&content_dir.join("acase_checksum_song.pak.xen"));
        write_test_file(&music_dir.join("case_checksum_preview.fsb.xen"));
        write_test_file(&music_dir.join("case_checksum_1.fsb.xen"));
        write_test_file(&music_dir.join("case_checksum_2.fsb.xen"));
        write_test_file(&music_dir.join("case_checksum_3.fsb.xen"));
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert!(result.content_file_issues.is_empty());
        assert!(project.mods_dir.join("content").join("music").is_dir());
        assert!(!project.mods_dir.join("Content").exists());
    }

    #[test]
    fn song_scan_accepts_checksum_file_casing_mismatches() {
        let project = TestProject::new("song-scan-content-file-case");
        let lower_song_dir = project.mods_dir.join("lower-files");
        let upper_song_dir = project.mods_dir.join("upper-files");
        fs::create_dir_all(&lower_song_dir).expect("lower song dir should be created");
        fs::create_dir_all(&upper_song_dir).expect("upper song dir should be created");
        write_test_file_contents(
            &lower_song_dir.join("song.ini"),
            valid_song_ini("Upper Checksum", "CASE_CHECKSUM").as_str(),
        );
        write_valid_content_files(&lower_song_dir, "case_checksum");
        write_test_file_contents(
            &upper_song_dir.join("song.ini"),
            valid_song_ini("Lower Checksum", "reverse_checksum").as_str(),
        );
        write_valid_content_files(&upper_song_dir, "REVERSE_CHECKSUM");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert!(result.content_file_issues.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn song_scan_suppresses_missing_music_folder_in_debug_builds() {
        let project = TestProject::new("song-scan-debug-missing-music");
        let content_dir = project.mods_dir.join("Content");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "debug_checksum").as_str(),
        );
        fs::create_dir_all(&content_dir).expect("content dir should be created");
        write_test_file(&content_dir.join("adebug_checksum_song.pak.xen"));
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert!(result.content_file_issues.is_empty());
    }

    #[test]
    fn song_scan_reports_missing_required_content_files_with_absolute_paths() {
        let project = TestProject::new("song-scan-content-missing-file");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "missing_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "missing_checksum");
        fs::remove_file(
            project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("missing_checksum_3.fsb.xen"),
        )
        .expect("required file should be removed");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.content_file_issues.len(), 1);
        assert_eq!(
            result.content_file_issues[0].message,
            "Missing required file."
        );
        assert!(result.content_file_issues[0]
            .absolute_path
            .ends_with("Content/MUSIC/missing_checksum_3.fsb.xen"));
        assert!(Path::new(&result.content_file_issues[0].absolute_path).is_absolute());
    }

    #[test]
    fn song_scan_reports_wrong_checksum_and_malformed_content_file_names() {
        let project = TestProject::new("song-scan-content-wrong-files");
        let content_dir = project.mods_dir.join("Content");
        let music_dir = content_dir.join("MUSIC");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "expected_checksum").as_str(),
        );
        fs::create_dir_all(&music_dir).expect("music dir should be created");
        write_test_file(&content_dir.join("aother_checksum_song.pak.xen"));
        write_test_file(&music_dir.join("expected_checksum_preview.fsb.xen"));
        write_test_file(&music_dir.join("expected_checksum_1.fsb.xen"));
        write_test_file(&music_dir.join("expected_checksum_2fsb.xen"));
        write_test_file(&music_dir.join("expected_checksum_3.fsb.xen"));
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Missing required file."
                && issue
                    .absolute_path
                    .ends_with("aexpected_checksum_song.pak.xen")
        }));
        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Missing required file."
                && issue.absolute_path.ends_with("expected_checksum_2.fsb.xen")
        }));
        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Unexpected file or folder."
                && issue
                    .absolute_path
                    .ends_with("aother_checksum_song.pak.xen")
        }));
        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Unexpected file or folder."
                && issue.absolute_path.ends_with("expected_checksum_2fsb.xen")
        }));
    }

    #[test]
    fn song_scan_reports_extra_content_and_music_files() {
        let project = TestProject::new("song-scan-content-extra-files");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "extra_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "extra_checksum");
        write_test_file(&project.mods_dir.join("Content").join("extra.txt"));
        write_test_file(
            &project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("extra.fsb.xen"),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(
            result
                .content_file_issues
                .iter()
                .filter(|issue| issue.message == "Unexpected file or folder.")
                .count(),
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn song_scan_reports_ambiguous_content_folder_casing() {
        let project = TestProject::new("song-scan-content-ambiguous-case");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Valid", "ambiguous_checksum").as_str(),
        );
        fs::create_dir_all(project.mods_dir.join("Content"))
            .expect("Content dir should be created");
        fs::create_dir_all(project.mods_dir.join("content"))
            .expect("content dir should be created");
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.content_file_issues.len(), 1);
        assert!(result.content_file_issues[0]
            .message
            .contains("Multiple folders match required Content folder casing"));
    }

    #[test]
    fn song_scan_ignores_disabled_song_ini_as_active_song() {
        let project = TestProject::new("song-scan-disabled-not-active");
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_found, 0);
        assert_eq!(result.songs_parsed, 0);
        assert!(result.duplicate_checksum_groups.is_empty());
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_scan_reports_active_and_disabled_sibling_conflict() {
        let project = TestProject::new("song-scan-disabled-conflict");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert_eq!(result.song_ini_folder_conflicts.len(), 1);
        assert_eq!(result.song_ini_folder_conflicts[0].folder_path, ".");
        assert_eq!(
            result.song_ini_folder_conflicts[0].file_paths,
            vec!["song.ini".to_string(), "song.disabled.ini".to_string()]
        );
    }

    #[test]
    fn song_validation_rejects_invalid_contents_without_writing() {
        let project = TestProject::new("song-validate-invalid");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            "[SongInfo\nTitle=Broken\n",
            &store,
        );

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_validation_rejects_duplicate_keys_without_writing() {
        let project = TestProject::new("song-validate-duplicates");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            "[ModInfo]\nName=Duplicate\n\n[SongInfo]\nChecksum=duplicate\nArtist=OK Go\nartist=OK Go\n",
            &store,
        );

        assert!(result
            .expect_err("duplicate keys should be rejected")
            .contains("duplicate key `artist`"));
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_validation_auto_corrects_known_keys_with_wrong_case() {
        let project = TestProject::new("song-validate-key-case");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nartist=Motorhead\n",
            &store,
        );

        let result = result.expect("wrong key case should be corrected");
        let expected_contents =
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nArtist=Motorhead\n";
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.contents, expected_contents);
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            expected_contents
        );
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].sections[1].entries[1].key, "Artist");
    }

    #[test]
    fn song_validation_rejects_missing_required_fields_without_writing() {
        let project = TestProject::new("song-validate-required-fields");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            "[ModInfo]\nName=No Checksum\n\n[SongInfo]\nTitle=No Checksum\n",
            &store,
        );

        assert!(result
            .expect_err("missing checksum should be rejected")
            .contains("missing required Checksum entry in [SongInfo]"));
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_validation_writes_valid_contents_with_faulty_backup_and_updates_store() {
        let project = TestProject::new("song-validate-valid");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_song_ini = valid_song_ini("Original", "original_checksum");
        write_test_file_contents(&song_ini_path, original_song_ini.as_str());
        let store = SongIniStore::default();

        let repaired_song_ini = valid_song_ini("Repaired", "repaired_checksum");
        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            &repaired_song_ini,
            &store,
        )
        .expect("validation should pass");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.songs_parsed, 1);
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            repaired_song_ini
        );
        assert!(!project.mods_dir.join("song.original.ini").exists());
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.faulty.ini"))
                .expect("song.original.faulty.ini should read"),
            original_song_ini
        );
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].sections[1].entries[1].value, "Repaired");
    }

    #[test]
    fn song_validation_returns_refreshed_display_rows_after_repair() {
        let project = TestProject::new("song-validate-refresh-display");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            "[ModInfo]\nName=Original\n\n[SongInfo]\nChecksum=display_checksum\nArtist=Original Artist\nTitle=Original Title\n",
        );
        let store = SongIniStore::default();

        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            "[ModInfo]\nName=Updated\n\n[SongInfo]\nChecksum=display_checksum\nArtist=Updated Artist\nTitle=Updated Title\nYear=1999\nGenre=Metal\nGameIcon=gh3\n",
            &store,
        )
        .expect("validation should pass");

        assert_eq!(result.songs.len(), 1);
        assert_eq!(result.songs[0].artist, "Updated Artist");
        assert_eq!(result.songs[0].title, "Updated Title");
        assert_eq!(result.songs[0].year, "1999");
        assert_eq!(result.songs[0].genre, "Metal");
        assert_eq!(result.songs[0].game_icon, "gh3");
    }

    #[test]
    fn song_validation_returns_refreshed_conflicts_after_repair() {
        let project = TestProject::new("song-validate-refresh-conflicts");
        let valid_dir = project.mods_dir.join("Valid");
        let repaired_dir = project.mods_dir.join("Repaired");
        fs::create_dir_all(&valid_dir).expect("valid song folder should be created");
        fs::create_dir_all(&repaired_dir).expect("repaired song folder should be created");
        write_test_file_contents(
            &valid_dir.join("song.ini"),
            valid_song_ini("Valid", "shared_checksum").as_str(),
        );
        write_test_file_contents(&repaired_dir.join("song.ini"), "[SongInfo\nTitle=Broken\n");
        write_test_file_contents(
            &repaired_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let scan_result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        assert!(scan_result.duplicate_checksum_groups.is_empty());
        assert_eq!(scan_result.song_ini_folder_conflicts.len(), 1);

        let validation_result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "Repaired/song.ini",
            &valid_song_ini("Repaired", "shared_checksum"),
            &store,
        )
        .expect("validation should pass");

        assert_eq!(validation_result.songs_parsed, 2);
        assert_eq!(validation_result.duplicate_checksum_groups.len(), 1);
        assert_eq!(
            validation_result.duplicate_checksum_groups[0].relative_paths,
            vec![
                "Repaired/song.ini".to_string(),
                "Valid/song.ini".to_string()
            ]
        );
        assert_eq!(validation_result.song_ini_folder_conflicts.len(), 1);
        assert_eq!(
            validation_result.song_ini_folder_conflicts[0].folder_path,
            "Repaired"
        );
    }

    #[test]
    fn song_validation_returns_refreshed_content_issues_after_checksum_change() {
        let project = TestProject::new("song-validate-refresh-content");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Original", "old_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "old_checksum");
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            &valid_song_ini("Updated", "new_checksum"),
            &store,
        )
        .expect("validation should pass");

        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Missing required file."
                && issue.absolute_path.ends_with("anew_checksum_song.pak.xen")
        }));
        assert!(result.content_file_issues.iter().any(|issue| {
            issue.message == "Unexpected file or folder."
                && issue.absolute_path.ends_with("aold_checksum_song.pak.xen")
        }));
    }

    #[test]
    fn song_repair_undo_restores_invalid_contents_and_removes_song_from_store() {
        let project = TestProject::new("song-repair-undo-invalid");
        let song_ini_path = project.mods_dir.join("song.ini");
        let invalid_contents = "[SongInfo\nTitle=Broken\n";
        let repaired_contents = valid_song_ini("Repaired", "repaired_checksum");
        write_test_file_contents(&song_ini_path, invalid_contents);
        let store = SongIniStore::default();

        validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            &repaired_contents,
            &store,
        )
        .expect("validation should pass");

        let result = undo_song_ini_repair_path(
            &test_scan_settings(&project),
            "song.ini",
            invalid_contents,
            &store,
        )
        .expect("undo should restore invalid contents");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.contents, invalid_contents);
        assert_eq!(result.songs_parsed, 0);
        assert!(result.songs.is_empty());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            invalid_contents
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_repair_undo_rejects_non_song_ini_path() {
        let project = TestProject::new("song-repair-undo-non-song");
        write_test_file_contents(&project.mods_dir.join("notes.ini"), "hello");
        let store = SongIniStore::default();

        let result = undo_song_ini_repair_path(
            &test_scan_settings(&project),
            "notes.ini",
            "restored",
            &store,
        );

        assert!(result
            .expect_err("non-song.ini path should be rejected")
            .contains("notes.ini is not a song.ini or song.excluded.ini file."));
    }

    #[test]
    fn song_repair_undo_rejects_unsafe_path() {
        let project = TestProject::new("song-repair-undo-unsafe");
        write_test_file_contents(&project.root.join("song.ini"), "outside");
        let store = SongIniStore::default();

        let result = undo_song_ini_repair_path(
            &test_scan_settings(&project),
            "../song.ini",
            "restored",
            &store,
        );

        assert!(result
            .expect_err("unsafe path should be rejected")
            .contains("Rejected unsafe MODS-relative path"));
    }

    #[test]
    fn content_issue_verify_refreshes_multiple_songs() {
        let project = TestProject::new("content-issue-verify-multiple");
        let first_song_path = project.mods_dir.join("song.ini");
        let second_song_path = project.mods_dir.join("Second").join("song.ini");
        write_test_file_contents(
            &first_song_path,
            valid_song_ini("First", "first_checksum").as_str(),
        );
        write_test_file_contents(
            &second_song_path,
            valid_song_ini("Second", "second_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "first_checksum");
        write_valid_content_files(&project.mods_dir.join("Second"), "second_checksum");
        fs::remove_file(
            project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("first_checksum_3.fsb.xen"),
        )
        .expect("first required file should be removed");
        fs::remove_file(
            project
                .mods_dir
                .join("Second")
                .join("Content")
                .join("MUSIC")
                .join("second_checksum_3.fsb.xen"),
        )
        .expect("second required file should be removed");
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);

        scan_song_ini_files_paths(&settings, &store).expect("scan should complete");
        write_test_file(
            &project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("first_checksum_3.fsb.xen"),
        );
        write_test_file(
            &project
                .mods_dir
                .join("Second")
                .join("Content")
                .join("MUSIC")
                .join("second_checksum_3.fsb.xen"),
        );

        let result = verify_content_issue_songs_paths(
            &settings,
            &["song.ini".to_string(), "Second/song.ini".to_string()],
            &store,
        )
        .expect("content issues should verify");

        assert_eq!(result.songs_parsed, 2);
        assert!(result.content_file_issues.is_empty());
    }

    #[test]
    fn content_issue_verify_removes_deleted_song_folder() {
        let project = TestProject::new("content-issue-verify-deleted-folder");
        let song_dir = project.mods_dir.join("Deleted");
        write_test_file_contents(
            &song_dir.join("song.ini"),
            valid_song_ini("Deleted", "deleted_checksum").as_str(),
        );
        write_valid_content_files(&song_dir, "deleted_checksum");
        fs::remove_file(
            song_dir
                .join("Content")
                .join("MUSIC")
                .join("deleted_checksum_3.fsb.xen"),
        )
        .expect("required file should be removed");
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);

        scan_song_ini_files_paths(&settings, &store).expect("scan should complete");
        fs::remove_dir_all(&song_dir).expect("song folder should be removed");

        let result =
            verify_content_issue_songs_paths(&settings, &["Deleted/song.ini".to_string()], &store)
                .expect("deleted song should be removed without an error");

        assert_eq!(result.songs_parsed, 0);
        assert!(result.songs.is_empty());
        assert!(result.content_file_issues.is_empty());
    }

    #[test]
    fn content_issue_verify_rejects_unsafe_paths() {
        let project = TestProject::new("content-issue-verify-unsafe-path");
        let store = SongIniStore::default();

        let error = verify_content_issue_songs_paths(
            &test_scan_settings(&project),
            &["../song.ini".to_string()],
            &store,
        )
        .expect_err("unsafe path should be rejected");

        assert!(error.contains("Rejected unsafe MODS-relative path"));
    }

    #[test]
    fn song_metadata_update_preserves_unrelated_ini_content() {
        let project = TestProject::new("song-metadata-update");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            "[ModInfo]\nName=Original Mod\n\n[SongInfo]\nChecksum=metadata_checksum\nTitle=Old Title\nArtist=Old Artist\nLeaderboard=Yes\n\n[Extra]\nValue=Keep\n",
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = update_scanned_song_metadata_path(
            &test_scan_settings(&project),
            "song.ini",
            ScannedSongMetadataInput {
                artist: "New Artist".to_string(),
                title: "New Title".to_string(),
                year: "2001".to_string(),
                genre: "Metal".to_string(),
                game_icon: "gh3".to_string(),
            },
            true,
            &store,
        )
        .expect("metadata update should pass");
        let contents = fs::read_to_string(&song_ini_path).expect("song.ini should read");

        assert!(contents.contains("Name=Original Mod"));
        assert!(contents.contains("Checksum=metadata_checksum"));
        assert!(contents.contains("Leaderboard=Yes"));
        assert!(contents.contains("[Extra]\nValue=Keep"));
        assert!(contents.contains("Artist=New Artist"));
        assert!(contents.contains("Title=New Title"));
        assert!(contents.contains("Year=2001"));
        assert!(contents.contains("Genre=Metal"));
        assert!(contents.contains("GameIcon=gh3"));
        assert_eq!(result.songs[0].artist, "New Artist");
        assert_eq!(result.songs[0].title, "New Title");
        assert!(result.songs[0].has_original_song_ini);
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("backup should read"),
            "[ModInfo]\nName=Original Mod\n\n[SongInfo]\nChecksum=metadata_checksum\nTitle=Old Title\nArtist=Old Artist\nLeaderboard=Yes\n\n[Extra]\nValue=Keep\n"
        );
    }

    #[test]
    fn song_metadata_update_removes_empty_song_info_entries() {
        let project = TestProject::new("song-metadata-remove-empty");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            "[ModInfo]\nName=Song\n\n[SongInfo]\nChecksum=metadata_checksum\nArtist=Artist\nTitle=Title\nYear=2001\nGenre=Rock\nGameIcon=gh3\n\n[Extra]\nValue=Keep\n",
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result = update_scanned_song_metadata_path(
            &settings,
            "song.ini",
            ScannedSongMetadataInput {
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                year: "\t".to_string(),
                genre: "".to_string(),
                game_icon: " ".to_string(),
            },
            true,
            &store,
        )
        .expect("empty metadata save should pass");
        let contents = fs::read_to_string(song_ini_path).expect("song.ini should read");

        for key in ["Year", "Genre", "GameIcon"] {
            assert!(!contents.contains(&format!("{key}=")));
        }
        assert!(contents.contains("Artist=Artist"));
        assert!(contents.contains("Title=Title"));
        assert!(contents.contains("Checksum=metadata_checksum"));
        assert!(contents.contains("[Extra]\nValue=Keep"));
        assert_eq!(result.songs[0].artist, "Artist");
        assert_eq!(result.songs[0].title, "Title");
        assert_eq!(result.songs[0].game_icon, "");
    }

    #[test]
    fn song_metadata_update_rejects_empty_artist_or_title() {
        let project = TestProject::new("song-metadata-required-fields");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_contents =
            "[ModInfo]\nName=Song\n\n[SongInfo]\nChecksum=metadata_checksum\nArtist=Artist\nTitle=Title\n";
        write_test_file_contents(&song_ini_path, original_contents);
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        for (artist, title, error) in [
            (" ", "Title", "Artist is required."),
            ("Artist", "\t", "Title is required."),
        ] {
            assert_eq!(
                update_scanned_song_metadata_path(
                    &settings,
                    "song.ini",
                    ScannedSongMetadataInput {
                        artist: artist.to_string(),
                        title: title.to_string(),
                        year: "".to_string(),
                        genre: "".to_string(),
                        game_icon: "".to_string(),
                    },
                    true,
                    &store,
                )
                .expect_err("empty required metadata should fail"),
                error
            );
        }
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            original_contents
        );
    }

    #[test]
    fn song_scan_includes_excluded_song_ini_files_with_unchecked_rows() {
        let project = TestProject::new("song-scan-includes-excluded");
        write_test_file_contents(
            &project.mods_dir.join("Active/song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("Excluded/SONG.EXCLUDED.INI"),
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should pass");

        assert_eq!(result.songs_found, 2);
        assert_eq!(result.songs_parsed, 2);
        assert_eq!(result.songs.len(), 2);
        assert!(result.songs.iter().any(|song| song.is_included));
        assert!(result.songs.iter().any(|song| !song.is_included));
    }

    #[test]
    fn song_scan_reports_active_and_excluded_sibling_conflict() {
        let project = TestProject::new("song-scan-active-excluded-conflict");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.excluded.ini"),
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should pass");

        assert_eq!(result.song_ini_folder_conflicts.len(), 1);
        assert_eq!(result.song_ini_folder_conflicts[0].folder_path, ".");
        assert_eq!(
            result.song_ini_folder_conflicts[0].file_paths,
            vec!["song.ini".to_string(), "song.excluded.ini".to_string()]
        );
    }

    #[test]
    fn song_scan_groups_all_song_ini_variants_per_folder() {
        let project = TestProject::new("song-scan-all-ini-variants");
        write_test_file_contents(
            &project.mods_dir.join("Nested/SONG.INI"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("Nested/song.excluded.ini"),
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("Nested/song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should pass");

        assert_eq!(result.song_ini_folder_conflicts.len(), 1);
        assert_eq!(result.song_ini_folder_conflicts[0].folder_path, "Nested");
        assert_eq!(
            result.song_ini_folder_conflicts[0].file_paths,
            vec![
                "Nested/SONG.INI".to_string(),
                "Nested/song.excluded.ini".to_string(),
                "Nested/song.disabled.ini".to_string(),
            ]
        );
    }

    #[test]
    fn song_scan_reports_excluded_and_disabled_sibling_conflict() {
        let project = TestProject::new("song-scan-excluded-disabled-conflict");
        write_test_file_contents(
            &project.mods_dir.join("song.excluded.ini"),
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should pass");

        assert_eq!(result.song_ini_folder_conflicts.len(), 1);
        assert_eq!(
            result.song_ini_folder_conflicts[0].file_paths,
            vec![
                "song.excluded.ini".to_string(),
                "song.disabled.ini".to_string()
            ]
        );
    }

    #[test]
    fn song_metadata_save_moves_excluded_song_to_selected_filename() {
        let project = TestProject::new("song-metadata-moves-excluded");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let active_path = project.mods_dir.join("song.ini");
        let original_contents = valid_song_ini("Excluded", "excluded_checksum");
        write_test_file_contents(&excluded_path, &original_contents);
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = update_scanned_song_metadata_path(
            &test_scan_settings(&project),
            "song.excluded.ini",
            ScannedSongMetadataInput {
                artist: "Artist".to_string(),
                title: "Included".to_string(),
                year: "2001".to_string(),
                genre: "Rock".to_string(),
                game_icon: "gh3".to_string(),
            },
            true,
            &store,
        )
        .expect("metadata save should pass");

        assert_eq!(result.relative_path, "song.ini");
        assert!(!excluded_path.exists());
        assert!(active_path.exists());
        assert!(fs::read_to_string(&active_path)
            .expect("active song should read")
            .contains("Title=Included"));
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("original backup should read"),
            original_contents
        );
    }

    #[test]
    fn song_metadata_save_rejects_existing_selected_filename_without_removing_source() {
        let project = TestProject::new("song-metadata-move-collision");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let active_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &excluded_path,
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(
            &active_path,
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = update_scanned_song_metadata_path(
            &test_scan_settings(&project),
            "song.excluded.ini",
            ScannedSongMetadataInput {
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                year: "2001".to_string(),
                genre: "Rock".to_string(),
                game_icon: "gh3".to_string(),
            },
            true,
            &store,
        )
        .expect_err("occupied destination should fail");

        assert!(result.contains("destination already exists"));
        assert!(excluded_path.exists());
        assert!(active_path.exists());
    }

    #[test]
    fn duplicate_exclusion_renames_active_song_to_excluded_file() {
        let project = TestProject::new("duplicate-exclusion-renames-song");
        let active_path = project.mods_dir.join("song.ini");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        write_test_file_contents(
            &active_path,
            valid_song_ini("Song", "song_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = set_scanned_song_included_path(
            &test_scan_settings(&project),
            "song.ini",
            false,
            &store,
        )
        .expect("exclusion should pass");

        assert_eq!(result.relative_path, "song.excluded.ini");
        assert!(!active_path.exists());
        assert!(excluded_path.exists());
        assert!(!result.songs[0].is_included);

        let result = set_scanned_song_included_path(
            &test_scan_settings(&project),
            "song.excluded.ini",
            true,
            &store,
        )
        .expect("inclusion should pass");

        assert_eq!(result.relative_path, "song.ini");
        assert!(active_path.exists());
        assert!(!excluded_path.exists());
        assert!(result.songs[0].is_included);
    }

    #[test]
    fn duplicate_inclusion_results_keep_renamed_member_in_checksum_group() {
        let project = TestProject::new("duplicate-inclusion-results");
        write_test_file_contents(
            &project.mods_dir.join("First/song.ini"),
            valid_song_ini("First", "shared_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("Second/song.ini"),
            valid_song_ini("Second", "shared_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = set_scanned_song_included_path(
            &test_scan_settings(&project),
            "First/song.ini",
            false,
            &store,
        )
        .expect("exclusion should pass");

        assert_eq!(result.duplicate_checksum_groups.len(), 1);
        assert_eq!(
            result.duplicate_checksum_groups[0].relative_paths,
            vec![
                "First/song.excluded.ini".to_string(),
                "Second/song.ini".to_string(),
            ]
        );

        let result = set_scanned_song_included_path(
            &test_scan_settings(&project),
            "First/song.excluded.ini",
            true,
            &store,
        )
        .expect("inclusion should pass");

        assert_eq!(
            result.duplicate_checksum_groups[0].relative_paths,
            vec!["First/song.ini".to_string(), "Second/song.ini".to_string()]
        );
    }

    #[test]
    fn song_metadata_update_creates_original_backup_when_faulty_backup_exists() {
        let project = TestProject::new("song-metadata-faulty-backup");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_song_ini = valid_song_ini("Original", "metadata_checksum");
        let faulty_backup = "[SongInfo\nTitle=Broken\n";
        write_test_file_contents(&song_ini_path, &original_song_ini);
        write_test_file_contents(
            &project.mods_dir.join("song.original.faulty.ini"),
            faulty_backup,
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = update_scanned_song_metadata_path(
            &test_scan_settings(&project),
            "song.ini",
            ScannedSongMetadataInput {
                artist: "New Artist".to_string(),
                title: "New Title".to_string(),
                year: "2001".to_string(),
                genre: "Metal".to_string(),
                game_icon: "gh3".to_string(),
            },
            true,
            &store,
        )
        .expect("metadata update should pass");

        assert!(result.songs[0].has_original_song_ini);
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("proper backup should read"),
            original_song_ini
        );
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.faulty.ini"))
                .expect("faulty backup should read"),
            faulty_backup
        );
    }

    #[test]
    fn song_metadata_update_rejects_unsafe_path() {
        let project = TestProject::new("song-metadata-unsafe");
        let outside_file = project.root.join("song.ini");
        write_test_file_contents(
            &outside_file,
            valid_song_ini("Outside", "outside_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = update_scanned_song_metadata_path(
            &test_scan_settings(&project),
            "../song.ini",
            ScannedSongMetadataInput {
                artist: "Artist".to_string(),
                title: "Title".to_string(),
                year: "2001".to_string(),
                genre: "Rock".to_string(),
                game_icon: "gh3".to_string(),
            },
            true,
            &store,
        )
        .expect_err("unsafe metadata update should fail");

        assert_eq!(result, "Rejected unsafe MODS-relative path ../song.ini.");
    }

    #[test]
    fn song_restore_original_consumes_backup_and_refreshes_store() {
        let project = TestProject::new("song-restore-original");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &backup_path,
            "[ModInfo]\nName=Original\n\n[SongInfo]\nChecksum=original_checksum\nArtist=Original Artist\nTitle=Original Title\nYear=1999\nGenre=Rock\nGameIcon=ghwt\n",
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result =
            restore_original_song_ini_path(&test_scan_settings(&project), "song.ini", true, &store)
                .expect("restore should pass");

        assert!(!backup_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            "[ModInfo]\nName=Original\n\n[SongInfo]\nChecksum=original_checksum\nArtist=Original Artist\nTitle=Original Title\nYear=1999\nGenre=Rock\nGameIcon=ghwt\n"
        );
        assert_eq!(result.songs.len(), 1);
        assert_eq!(result.songs[0].artist, "Original Artist");
        assert_eq!(result.songs[0].title, "Original Title");
        assert_eq!(result.songs[0].game_icon, "ghwt");
        assert!(!result.songs[0].has_original_song_ini);

        let stored_songs = store.0.lock().expect("store should lock");
        assert_eq!(
            stored_songs[0].sections[1].entries[0].value,
            "original_checksum"
        );
    }

    #[test]
    fn song_restore_original_moves_excluded_song_to_selected_filename() {
        let project = TestProject::new("song-restore-original-moves-excluded");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let active_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        let backup_contents = valid_song_ini("Original", "original_checksum");
        write_test_file_contents(
            &excluded_path,
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(&backup_path, &backup_contents);
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = restore_original_song_ini_path(
            &test_scan_settings(&project),
            "song.excluded.ini",
            true,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.relative_path, "song.ini");
        assert!(!excluded_path.exists());
        assert!(!backup_path.exists());
        assert_eq!(
            fs::read_to_string(active_path).expect("song.ini should read"),
            backup_contents
        );
    }

    #[test]
    fn song_restore_original_rejects_missing_backup() {
        let project = TestProject::new("song-restore-missing-original");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result =
            restore_original_song_ini_path(&test_scan_settings(&project), "song.ini", true, &store)
                .expect_err("restore without backup should fail");

        assert_eq!(
            result,
            "song.ini has no original song.ini backup to restore."
        );
    }

    #[test]
    fn song_restore_original_rejects_faulty_backup_only() {
        let project = TestProject::new("song-restore-faulty-only");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.original.faulty.ini"),
            "[SongInfo\nTitle=Broken\n",
        );
        let store = SongIniStore::default();

        let result =
            restore_original_song_ini_path(&test_scan_settings(&project), "song.ini", true, &store)
                .expect_err("restore without proper backup should fail");

        assert_eq!(
            result,
            "song.ini has no original song.ini backup to restore."
        );
        assert!(project.mods_dir.join("song.original.faulty.ini").exists());
    }

    #[test]
    fn song_restore_all_after_format_restores_original_backup() {
        let project = TestProject::new("song-restore-all-after-format");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        let faulty_path = project.mods_dir.join("song.original.faulty.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &backup_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(
            &faulty_path,
            valid_song_ini("Faulty Original", "faulty_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::AfterFormatIssueFixes,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!backup_path.exists());
        assert!(faulty_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_restore_all_reactivates_excluded_song_without_backup() {
        let project = TestProject::new("song-restore-all-reactivates-excluded");
        let song_ini_path = project.mods_dir.join("Nested/song.ini");
        let excluded_path = project.mods_dir.join("Nested/SONG.EXCLUDED.INI");
        let excluded_contents = valid_song_ini("Excluded", "excluded_checksum");
        write_test_file_contents(&excluded_path, &excluded_contents);
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::AfterFormatIssueFixes,
            &store,
        )
        .expect("restore should pass");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.files_restored, 1);
        assert!(!excluded_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            excluded_contents
        );
    }

    #[test]
    fn song_restore_all_reactivates_excluded_song_then_restores_original_backup() {
        let project = TestProject::new("song-restore-all-excluded-original");
        let song_ini_path = project.mods_dir.join("song.ini");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let original_path = project.mods_dir.join("song.original.ini");
        write_test_file_contents(
            &excluded_path,
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(
            &original_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::AfterFormatIssueFixes,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!excluded_path.exists());
        assert!(!original_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
    }

    #[test]
    fn song_restore_all_before_format_restores_original_then_faulty_and_removes_both_backups() {
        let project = TestProject::new("song-restore-all-before-format-faulty");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_path = project.mods_dir.join("song.original.ini");
        let faulty_path = project.mods_dir.join("song.original.faulty.ini");
        let faulty_contents = "[SongInfo\nTitle=Broken Before Fix\n";
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &original_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(&faulty_path, faulty_contents);
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::BeforeFormatIssueFix,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!original_path.exists());
        assert!(!faulty_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            faulty_contents
        );
    }

    #[test]
    fn song_restore_all_before_format_reactivates_excluded_song_then_restores_faulty_backup() {
        let project = TestProject::new("song-restore-all-excluded-faulty");
        let song_ini_path = project.mods_dir.join("song.ini");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let original_path = project.mods_dir.join("song.original.ini");
        let faulty_path = project.mods_dir.join("song.original.faulty.ini");
        let faulty_contents = "[SongInfo\nTitle=Broken Before Fix\n";
        write_test_file_contents(
            &excluded_path,
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(
            &original_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(&faulty_path, faulty_contents);
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::BeforeFormatIssueFix,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!excluded_path.exists());
        assert!(!original_path.exists());
        assert!(!faulty_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            faulty_contents
        );
    }

    #[test]
    fn song_restore_all_before_format_falls_back_to_original_backup() {
        let project = TestProject::new("song-restore-all-before-format-original");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(
            &backup_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::BeforeFormatIssueFix,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!backup_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "original_checksum")
        );
    }

    #[test]
    fn song_restore_all_restores_invalid_backup_as_plain_file_copy() {
        let project = TestProject::new("song-restore-all-plain-copy");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        let backup_contents = "[SongInfo\nTitle=Broken\n";
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        write_test_file_contents(&backup_path, backup_contents);
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::AfterFormatIssueFixes,
            &store,
        )
        .expect("restore should pass");

        assert_eq!(result.files_restored, 1);
        assert!(result.errors.is_empty());
        assert!(!backup_path.exists());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            backup_contents
        );
    }

    #[test]
    fn restore_ini_deletes_recursive_instrument_sidecars_case_insensitively() {
        let project = TestProject::new("restore-ini-delete-instrument-sidecars");
        let active_song_ini = project.mods_dir.join("Active/song.ini");
        let active_sidecar = project.mods_dir.join("Active/song.instruments.ini");
        let excluded_song_ini = project.mods_dir.join("Excluded/song.excluded.ini");
        let excluded_sidecar = project.mods_dir.join("Excluded/SONG.INSTRUMENTS.INI");
        let unrelated_file = project.mods_dir.join("Excluded/notes.ini");
        write_test_file_contents(
            &active_song_ini,
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(&active_sidecar, "[Cache]\n");
        write_test_file_contents(
            &excluded_song_ini,
            valid_song_ini("Excluded", "excluded_checksum").as_str(),
        );
        write_test_file_contents(&excluded_sidecar, "[Cache]\n");
        write_test_file_contents(&unrelated_file, "keep");
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::DeleteInstrumentSidecars,
            &store,
        )
        .expect("sidecar deletion should pass");

        assert_eq!(result.files_restored, 0);
        assert_eq!(result.files_deleted, 2);
        assert!(result.errors.is_empty());
        assert!(active_song_ini.exists());
        assert!(excluded_song_ini.exists());
        assert!(!active_sidecar.exists());
        assert!(!excluded_sidecar.exists());
        assert!(unrelated_file.exists());
    }

    #[test]
    fn restore_ini_deletes_no_instrument_sidecars_when_none_exist() {
        let project = TestProject::new("restore-ini-delete-no-instrument-sidecars");
        write_test_file_contents(&project.mods_dir.join("song.ini"), "[SongInfo]\n");
        let store = SongIniStore::default();

        let result = restore_all_ini_paths(
            &test_scan_settings(&project),
            RestoreIniAction::DeleteInstrumentSidecars,
            &store,
        )
        .expect("sidecar deletion should pass");

        assert_eq!(result.files_restored, 0);
        assert_eq!(result.files_deleted, 0);
        assert!(result.errors.is_empty());
        assert!(project.mods_dir.join("song.ini").exists());
    }

    #[test]
    fn song_validation_preserves_existing_original_backup() {
        let project = TestProject::new("song-validate-existing-backup");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        let existing_backup = valid_song_ini("Earlier Backup", "earlier_checksum");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(&backup_path, &existing_backup);
        let store = SongIniStore::default();

        validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            &valid_song_ini("Repaired", "repaired_checksum"),
            &store,
        )
        .expect("validation should pass");

        assert_eq!(
            fs::read_to_string(backup_path).expect("backup should read"),
            existing_backup
        );
    }

    #[test]
    fn song_validation_skips_original_backup_when_setting_is_disabled() {
        let project = TestProject::new("song-validate-no-backup");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        validate_song_ini_file_path(
            &test_scan_settings_without_song_ini_backup(&project),
            "song.ini",
            &valid_song_ini("Repaired", "repaired_checksum"),
            &store,
        )
        .expect("validation should pass");

        assert!(!project.mods_dir.join("song.original.ini").exists());
        assert!(!project.mods_dir.join("song.original.faulty.ini").exists());
    }

    #[cfg(unix)]
    #[test]
    fn song_validation_does_not_write_when_original_backup_fails() {
        let project = TestProject::new("song-validate-backup-fails");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_song_ini = valid_song_ini("Original", "original_checksum");
        write_test_file_contents(&song_ini_path, &original_song_ini);
        fs::set_permissions(&project.mods_dir, fs::Permissions::from_mode(0o500))
            .expect("mods dir should become read-only");
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "song.ini",
            &valid_song_ini("Repaired", "repaired_checksum"),
            &store,
        );

        fs::set_permissions(&project.mods_dir, fs::Permissions::from_mode(0o700))
            .expect("mods dir should become writable for cleanup");
        assert!(result
            .expect_err("backup failure should reject write")
            .contains("Failed to write"));
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            original_song_ini
        );
        assert!(!project.mods_dir.join("song.original.ini").exists());
        assert!(!project.mods_dir.join("song.original.faulty.ini").exists());
    }

    #[test]
    fn song_validation_rejects_paths_outside_mods() {
        let project = TestProject::new("song-validate-outside");
        let outside_file = project.root.join("song.ini");
        write_test_file_contents(&outside_file, "[SongInfo]\nTitle=Outside\n");
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &test_scan_settings(&project),
            "../song.ini",
            "[SongInfo]\nTitle=Unsafe\n",
            &store,
        );

        assert_eq!(
            result.expect_err("unsafe path should be rejected"),
            "Rejected unsafe MODS-relative path ../song.ini."
        );
        assert_eq!(
            fs::read_to_string(outside_file).expect("outside file should read"),
            "[SongInfo]\nTitle=Outside\n"
        );
    }

    #[test]
    fn song_disable_saves_original_renames_file_and_updates_store() {
        let project = TestProject::new("song-disable");
        let song_ini_path = project.mods_dir.join("song.ini");
        let original_song_ini = valid_song_ini("Original", "original_checksum");
        write_test_file_contents(&song_ini_path, &original_song_ini);
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let result = disable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("disable should succeed");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.disabled_path, "song.disabled.ini");
        assert!(result.is_included);
        assert_eq!(result.songs_parsed, 0);
        assert!(!song_ini_path.exists());
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.disabled.ini"))
                .expect("disabled song should read"),
            original_song_ini
        );
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("original backup should read"),
            original_song_ini
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_disable_preserves_existing_original_backup() {
        let project = TestProject::new("song-disable-existing-backup");
        let song_ini_path = project.mods_dir.join("song.ini");
        let backup_path = project.mods_dir.join("song.original.ini");
        let existing_backup = valid_song_ini("Earlier Backup", "earlier_checksum");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(&backup_path, &existing_backup);
        let store = SongIniStore::default();

        disable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("disable should succeed");

        assert_eq!(
            fs::read_to_string(backup_path).expect("backup should read"),
            existing_backup
        );
    }

    #[test]
    fn song_disable_and_enable_preserves_excluded_filename() {
        let project = TestProject::new("song-disable-enable-excluded");
        let excluded_path = project.mods_dir.join("song.excluded.ini");
        let disabled_path = project.mods_dir.join("song.disabled.ini");
        let excluded_song_ini = valid_song_ini("Excluded", "excluded_checksum");
        write_test_file_contents(&excluded_path, &excluded_song_ini);
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate store");

        let disabled =
            disable_song_ini_file_path(&test_scan_settings(&project), "song.excluded.ini", &store)
                .expect("disable should succeed");

        assert!(!disabled.is_included);
        assert!(!excluded_path.exists());
        assert!(disabled_path.exists());

        let enabled =
            enable_song_ini_file_path(&test_scan_settings(&project), "song.excluded.ini", &store)
                .expect("enable should succeed");

        assert_eq!(enabled.enabled_path, "song.excluded.ini");
        assert!(!enabled.is_included);
        assert!(!disabled_path.exists());
        assert_eq!(
            fs::read_to_string(excluded_path).expect("excluded song should read"),
            excluded_song_ini
        );
        assert_eq!(store.0.lock().expect("store should lock").len(), 1);
    }

    #[test]
    fn song_disable_rejects_unsafe_path_and_existing_disabled_file() {
        let project = TestProject::new("song-disable-rejects");
        let outside_file = project.root.join("song.ini");
        write_test_file_contents(&outside_file, valid_song_ini("Outside", "outside").as_str());
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        assert_eq!(
            disable_song_ini_file_path(&test_scan_settings(&project), "../song.ini", &store)
                .expect_err("unsafe path should be rejected"),
            "Rejected unsafe MODS-relative path ../song.ini."
        );
        assert!(
            disable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
                .expect_err("existing disabled file should be rejected")
                .contains("song.disabled.ini already exists")
        );
        assert!(project.mods_dir.join("song.ini").exists());
        assert!(outside_file.exists());
    }

    #[test]
    fn song_enable_renames_disabled_file_and_updates_store_when_valid() {
        let project = TestProject::new("song-enable");
        let disabled_song_ini = valid_song_ini("Enabled", "enabled_checksum");
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            &disabled_song_ini,
        );
        let store = SongIniStore::default();

        let result = enable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("enable should succeed");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.enabled_path, "song.ini");
        assert!(result.is_included);
        assert_eq!(result.songs_parsed, 1);
        assert!(!project.mods_dir.join("song.disabled.ini").exists());
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.ini"))
                .expect("enabled song should read"),
            disabled_song_ini
        );
        assert_eq!(store.0.lock().expect("store should lock").len(), 1);
    }

    #[test]
    fn song_enable_allows_invalid_disabled_file_without_updating_store() {
        let project = TestProject::new("song-enable-invalid");
        let disabled_song_ini = "[SongInfo]\nArtist=No title\n";
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            disabled_song_ini,
        );
        let store = SongIniStore::default();

        let result = enable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("enable should succeed");

        assert_eq!(result.songs_parsed, 0);
        assert!(!project.mods_dir.join("song.disabled.ini").exists());
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.ini"))
                .expect("enabled song should read"),
            disabled_song_ini
        );
        assert!(store.0.lock().expect("store should lock").is_empty());
    }

    #[test]
    fn song_enable_rejects_opposite_active_or_excluded_sibling() {
        let project = TestProject::new("song-enable-opposite-sibling");
        write_test_file_contents(
            &project.mods_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result =
            enable_song_ini_file_path(&test_scan_settings(&project), "song.excluded.ini", &store)
                .expect_err("opposite sibling should reject enable");

        assert!(result.contains("song.ini already exists"));
        assert!(project.mods_dir.join("song.disabled.ini").exists());
    }

    #[test]
    fn song_enable_rejects_unsafe_enabled_and_missing_disabled_paths() {
        let project = TestProject::new("song-enable-rejects");
        let outside_file = project.root.join("song.disabled.ini");
        let enabled_dir = project.mods_dir.join("AlreadyEnabled");
        write_test_file_contents(&outside_file, valid_song_ini("Outside", "outside").as_str());
        fs::create_dir_all(&enabled_dir).expect("enabled folder should be created");
        write_test_file_contents(
            &enabled_dir.join("song.ini"),
            valid_song_ini("Enabled", "enabled_checksum").as_str(),
        );
        let store = SongIniStore::default();

        assert_eq!(
            enable_song_ini_file_path(&test_scan_settings(&project), "../song.ini", &store)
                .expect_err("unsafe path should be rejected"),
            "Rejected unsafe MODS-relative path ../song.ini."
        );
        assert_eq!(
            enable_song_ini_file_path(
                &test_scan_settings(&project),
                "AlreadyEnabled/song.ini",
                &store,
            )
            .expect_err("enabled song should be rejected"),
            "AlreadyEnabled/song.ini is already enabled."
        );
        assert_eq!(
            enable_song_ini_file_path(&test_scan_settings(&project), "Missing/song.ini", &store)
                .expect_err("missing disabled song should be rejected"),
            "Missing/song.ini has no disabled song.ini file to enable."
        );
        assert!(outside_file.exists());
    }

    #[test]
    fn song_conflict_delete_removes_selected_active_or_disabled_file() {
        let project = TestProject::new("song-conflict-delete");
        let active_dir = project.mods_dir.join("Active");
        let disabled_dir = project.mods_dir.join("Disabled");
        fs::create_dir_all(&active_dir).expect("active folder should be created");
        fs::create_dir_all(&disabled_dir).expect("disabled folder should be created");
        write_test_file_contents(
            &active_dir.join("song.ini"),
            valid_song_ini("Active", "active_checksum").as_str(),
        );
        write_test_file_contents(
            &disabled_dir.join("song.disabled.ini"),
            valid_song_ini("Disabled", "disabled_checksum").as_str(),
        );
        let store = SongIniStore::default();
        scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should populate active song");

        let active_result = delete_song_ini_conflict_file_path(
            &test_scan_settings(&project),
            "Active/song.ini",
            &store,
        )
        .expect("active delete should succeed");
        let disabled_result = delete_song_ini_conflict_file_path(
            &test_scan_settings(&project),
            "Disabled/song.disabled.ini",
            &store,
        )
        .expect("disabled delete should succeed");

        assert_eq!(active_result.songs_parsed, 0);
        assert_eq!(disabled_result.songs_parsed, 0);
        assert!(!active_dir.join("song.ini").exists());
        assert!(!disabled_dir.join("song.disabled.ini").exists());
    }

    #[test]
    fn song_conflict_delete_rejects_unsafe_and_non_song_paths() {
        let project = TestProject::new("song-conflict-delete-rejects");
        let outside_file = project.root.join("song.ini");
        write_test_file_contents(&outside_file, valid_song_ini("Outside", "outside").as_str());
        write_test_file_contents(&project.mods_dir.join("notes.txt"), "notes");
        let store = SongIniStore::default();

        assert_eq!(
            delete_song_ini_conflict_file_path(
                &test_scan_settings(&project),
                "../song.ini",
                &store
            )
            .expect_err("unsafe path should be rejected"),
            "Rejected unsafe MODS-relative path ../song.ini."
        );
        assert_eq!(
            delete_song_ini_conflict_file_path(&test_scan_settings(&project), "notes.txt", &store)
                .expect_err("non-song path should be rejected"),
            "notes.txt is not a song.ini, song.excluded.ini, or song.disabled.ini file."
        );
        assert!(outside_file.exists());
        assert!(project.mods_dir.join("notes.txt").exists());
    }

    #[test]
    fn game_icon_scan_discovers_case_insensitive_gamelogos_and_matching_category() {
        let project = TestProject::new("game-icon-scan");
        let folder = project.mods_dir.join("Icons");
        write_test_file_contents(&folder.join("GAMELOGO_BH.IMG.XEN"), "logo");
        write_test_file_contents(
            &folder.join("CATEGORY.INI"),
            "[CategoryInfo]\nLogo=gamelogo_bh\n",
        );

        let settings = test_scan_settings(&project);
        let result =
            scan_game_icon_categories_paths(&settings).expect("game icon scan should succeed");

        assert!(!result.needs_fix);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].folder_relative_path, "Icons");
        assert!(result.groups[0].gamelogos[0].has_matching_category_ini);
        assert_eq!(result.custom_game_icons, vec!["GAMELOGO_BH".to_string()]);
    }

    #[test]
    fn game_icon_scan_skips_top_level_ini_tool_categories_only() {
        let project = TestProject::new("game-icon-scan-skip-ini-tool-categories");
        write_test_file_contents(
            &project
                .mods_dir
                .join("INIToolCATEGORIES/Category_01/gamelogo_generated.img.xen"),
            "generated",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack/IniToolCategories/gamelogo_custom.img.xen"),
            "custom",
        );

        let settings = test_scan_settings(&project);
        let result =
            scan_game_icon_categories_paths(&settings).expect("game icon scan should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(
            result.groups[0].folder_relative_path,
            "Pack/IniToolCategories"
        );
        assert_eq!(
            result.custom_game_icons,
            vec!["gamelogo_custom".to_string()]
        );
        assert_eq!(
            game_icon_valid_stems(&settings),
            vec!["gamelogo_custom".to_string()]
        );
    }

    #[test]
    fn game_icon_fix_moves_multiple_gamelogos_into_category_folders() {
        let project = TestProject::new("game-icon-fix-multiple");
        let folder = project.mods_dir.join("Pack");
        write_test_file_contents(&folder.join("gamelogo_apocalypse.img.xen"), "apocalypse");
        write_test_file_contents(&folder.join("gamelogo_bh.img.xen"), "bh");
        write_test_file_contents(
            &folder.join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_bh\n",
        );

        let result = fix_game_icon_categories_paths(&test_scan_settings(&project))
            .expect("game icon fix should succeed");

        assert!(!result.needs_fix);
        assert!(folder
            .join("Category_gamelogo_apocalypse")
            .join("gamelogo_apocalypse.img.xen")
            .is_file());
        assert!(folder
            .join("Category_gamelogo_bh")
            .join("gamelogo_bh.img.xen")
            .is_file());
        assert!(folder
            .join("Category_gamelogo_bh")
            .join("category.ini")
            .is_file());
        assert!(!folder.join("category.ini").exists());
    }

    #[test]
    fn game_icon_fix_renames_invalid_category_and_regenerates() {
        let project = TestProject::new("game-icon-fix-invalid");
        let folder = project.mods_dir.join("Broken");
        write_test_file_contents(&folder.join("gamelogo_apocalypse.img.xen"), "logo");
        write_test_file_contents(
            &folder.join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_old\nLogo=gamelogo_apocalypse\n",
        );

        let result = fix_game_icon_categories_paths(&test_scan_settings(&project))
            .expect("game icon fix should succeed");

        assert!(!result.needs_fix);
        assert!(folder.join("category.original.faulty.ini").is_file());
        assert_eq!(
            fs::read_to_string(folder.join("category.ini")).expect("category.ini should read"),
            "[ModInfo]\nName=GameIcon apocalypse Category\nDescription=Used for GameIcon, no songs linked\nAuthor=GhwtDeIniTool\nVersion=1.0\n\n[CategoryInfo]\nName=GameIcon apocalypse\nChecksum=apocalypse\nLogo=gamelogo_apocalypse\n"
        );
    }

    #[test]
    fn game_icon_fix_regenerates_missing_and_mismatched_category() {
        let project = TestProject::new("game-icon-fix-missing-mismatch");
        let missing_folder = project.mods_dir.join("Missing");
        let mismatch_folder = project.mods_dir.join("Mismatch");
        write_test_file_contents(&missing_folder.join("gamelogo_missing.img.xen"), "logo");
        write_test_file_contents(&mismatch_folder.join("gamelogo_mismatch.img.xen"), "logo");
        write_test_file_contents(
            &mismatch_folder.join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_other\n",
        );

        let result = fix_game_icon_categories_paths(&test_scan_settings(&project))
            .expect("game icon fix should succeed");

        assert!(!result.needs_fix);
        assert!(missing_folder.join("category.ini").is_file());
        assert!(mismatch_folder
            .join("category.original.faulty.ini")
            .is_file());
        assert!(fs::read_to_string(mismatch_folder.join("category.ini"))
            .expect("category.ini should read")
            .contains("Logo=gamelogo_mismatch"));
    }

    #[test]
    fn game_icon_fix_reports_collision_without_overwrite() {
        let project = TestProject::new("game-icon-fix-collision");
        let folder = project.mods_dir.join("Pack");
        write_test_file_contents(&folder.join("gamelogo_bh.img.xen"), "source");
        write_test_file_contents(&folder.join("gamelogo_rr.img.xen"), "other");
        write_test_file_contents(
            &folder
                .join("Category_gamelogo_bh")
                .join("gamelogo_bh.img.xen"),
            "target",
        );

        let result = fix_game_icon_categories_paths(&test_scan_settings(&project))
            .expect("game icon fix should succeed");

        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("already exists")));
        assert_eq!(
            fs::read_to_string(
                folder
                    .join("Category_gamelogo_bh")
                    .join("gamelogo_bh.img.xen")
            )
            .expect("target gamelogo should read"),
            "target"
        );
    }

    #[test]
    fn game_icon_scan_returns_official_and_custom_stems() {
        let project = TestProject::new("game-icon-stems");
        let official_dir = project.root.join("DATA").join("IMAGES").join("GAMELOGOS");
        write_test_file_contents(&project.mods_dir.join("gamelogo_custom.img.xen"), "custom");
        write_test_file_contents(&official_dir.join("gamelogo_gh3.img.xen"), "official");
        write_test_file_contents(&official_dir.join("notes.txt"), "ignored");
        let settings = ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: Some(official_dir),
            keep_original_song_ini: true,
        };

        let result = scan_game_icon_categories_paths(&settings).expect("scan should succeed");

        assert_eq!(
            result.custom_game_icons,
            vec!["gamelogo_custom".to_string()]
        );
        assert_eq!(result.official_game_icons, vec!["gamelogo_gh3".to_string()]);
    }

    #[test]
    fn game_icon_song_fix_preview_accepts_official_and_custom_icons() {
        let project = TestProject::new("game-icon-song-fix-valid-icons");
        let official_dir = project.root.join("DATA").join("IMAGES").join("GAMELOGOS");
        write_test_file_contents(&official_dir.join("gamelogo_gh3.img.xen"), "official");
        write_test_file_contents(
            &project
                .mods_dir
                .join("Icons")
                .join("gamelogo_custom.img.xen"),
            "custom",
        );
        write_test_file_contents(
            &project.mods_dir.join("A").join("Official").join("song.ini"),
            &song_ini_with_game_icon("Official", "official_checksum", "gamelogo_gh3"),
        );
        write_test_file_contents(
            &project.mods_dir.join("A").join("Custom").join("song.ini"),
            &song_ini_with_game_icon("Custom", "custom_checksum", "gamelogo_custom"),
        );
        write_test_file_contents(
            &project.mods_dir.join("A").join("Bad").join("song.ini"),
            &song_ini_with_game_icon("Bad", "bad_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: Some(official_dir),
            keep_original_song_ini: true,
        };
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].relative_path, "A/Bad/song.ini");
        assert_eq!(result.rows[0].parent_relative_path, "A");
        assert_eq!(result.rows[0].invalid_game_icon, "gamelogo_missing");
        assert!(result
            .valid_game_icons
            .iter()
            .any(|icon| icon == "gamelogo_gh3"));
        assert!(result
            .valid_game_icons
            .iter()
            .any(|icon| icon == "gamelogo_custom"));
    }

    #[test]
    fn game_icon_song_fix_preview_uses_parent_folder_majority() {
        let project = TestProject::new("game-icon-song-fix-majority");
        let official_dir = project.root.join("DATA").join("IMAGES").join("GAMELOGOS");
        write_test_file_contents(&official_dir.join("gamelogo_a.img.xen"), "a");
        write_test_file_contents(&official_dir.join("gamelogo_b.img.xen"), "b");
        write_test_file_contents(
            &project.mods_dir.join("folder0/folder1/folder1/song.ini"),
            &song_ini_with_game_icon("One", "one_checksum", "gamelogo_a"),
        );
        write_test_file_contents(
            &project.mods_dir.join("folder0/folder1/folder2/song.ini"),
            &song_ini_with_game_icon("Two", "two_checksum", "gamelogo_a"),
        );
        write_test_file_contents(
            &project.mods_dir.join("folder0/folder1/folder3/song.ini"),
            &song_ini_with_game_icon("Three", "three_checksum", "gamelogo_b"),
        );
        write_test_file_contents(
            &project.mods_dir.join("folder0/folder1/folder4/song.ini"),
            &song_ini_with_game_icon("Bad", "bad_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: Some(official_dir),
            keep_original_song_ini: true,
        };
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "gamelogo_a");
    }

    #[test]
    fn game_icon_song_fix_preview_keeps_first_icon_on_majority_tie() {
        let project = TestProject::new("game-icon-song-fix-tie");
        let official_dir = project.root.join("DATA").join("IMAGES").join("GAMELOGOS");
        write_test_file_contents(&official_dir.join("gamelogo_a.img.xen"), "a");
        write_test_file_contents(&official_dir.join("gamelogo_b.img.xen"), "b");
        write_test_file_contents(
            &project.mods_dir.join("Pack/First/song.ini"),
            &song_ini_with_game_icon("First", "first_checksum", "gamelogo_b"),
        );
        write_test_file_contents(
            &project.mods_dir.join("Pack/Second/song.ini"),
            &song_ini_with_game_icon("Second", "second_checksum", "gamelogo_a"),
        );
        write_test_file_contents(
            &project.mods_dir.join("Pack/Third/song.ini"),
            &song_ini_with_game_icon("Third", "third_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = ScanSettings {
            mods_dir: project.mods_dir.clone(),
            official_gamelogos_dir: Some(official_dir),
            keep_original_song_ini: true,
        };
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows[0].new_game_icon, "gamelogo_b");
    }

    #[test]
    fn game_icon_song_fix_preview_does_not_fall_back_to_custom_folder_ancestor() {
        let project = TestProject::new("game-icon-song-fix-custom-ancestor");
        write_test_file_contents(
            &project.mods_dir.join("Pack").join("gamelogo_pack.img.xen"),
            "pack",
        );
        write_test_file_contents(
            &project.mods_dir.join("Pack").join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_pack\n",
        );
        write_test_file_contents(
            &project.mods_dir.join("Pack/Artist/Song/song.ini"),
            &song_ini_with_game_icon("Song", "song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "");
    }

    #[test]
    fn game_icon_song_fix_preview_falls_back_to_direct_child_custom_folder() {
        let project = TestProject::new("game-icon-song-fix-child-category");
        write_test_file_contents(
            &project
                .mods_dir
                .join("Rock Band Network!!!!!")
                .join("Version 1")
                .join("Category_RBNV1")
                .join("gamelogo_rbnv1.img.xen"),
            "rbnv1",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Rock Band Network!!!!!")
                .join("Version 1")
                .join("Category_RBNV1")
                .join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_rbnv1\n",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Rock Band Network!!!!!")
                .join("Version 1")
                .join("Some Song")
                .join("song.ini"),
            &song_ini_with_game_icon("Some Song", "some_song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "gamelogo_rbnv1");
    }

    #[test]
    fn game_icon_song_fix_preview_uses_first_sorted_direct_child_custom_folder() {
        let project = TestProject::new("game-icon-song-fix-child-category-first");
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack")
                .join("Category_B")
                .join("gamelogo_b.img.xen"),
            "b",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack")
                .join("Category_B")
                .join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_b\n",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack")
                .join("Category_A")
                .join("gamelogo_a.img.xen"),
            "a",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack")
                .join("Category_A")
                .join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_a\n",
        );
        write_test_file_contents(
            &project
                .mods_dir
                .join("Pack")
                .join("Some Song")
                .join("song.ini"),
            &song_ini_with_game_icon("Some Song", "some_song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "gamelogo_a");
    }

    #[test]
    fn game_icon_song_fix_preview_uses_mods_folder_for_root_song() {
        let project = TestProject::new("game-icon-song-fix-root-song");
        write_test_file_contents(&project.mods_dir.join("gamelogo_root.img.xen"), "root");
        write_test_file_contents(
            &project.mods_dir.join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_root\n",
        );
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            &song_ini_with_game_icon("Song", "song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "gamelogo_root");
    }

    #[test]
    fn game_icon_song_fix_preview_requires_matching_category() {
        let project = TestProject::new("game-icon-song-fix-matching-category");
        let folder = project.mods_dir.join("Pack");
        write_test_file_contents(&folder.join("gamelogo_missing_category.img.xen"), "missing");
        write_test_file_contents(&folder.join("gamelogo_mismatched.img.xen"), "mismatched");
        write_test_file_contents(
            &folder.join("category.ini"),
            "[CategoryInfo]\nLogo=gamelogo_other\n",
        );
        write_test_file_contents(
            &folder.join("Song/song.ini"),
            &song_ini_with_game_icon("Song", "song_checksum", "gamelogo_invalid"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result =
            preview_game_icon_song_fixes_paths(&settings, &store).expect("preview should pass");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].new_game_icon, "");
    }

    #[test]
    fn game_icon_song_fix_apply_rejects_unknown_replacements() {
        let project = TestProject::new("game-icon-song-fix-reject-unknown");
        write_test_file_contents(
            &project
                .mods_dir
                .join("Icons")
                .join("gamelogo_known.img.xen"),
            "known",
        );
        let song_ini_path = project.mods_dir.join("Song").join("song.ini");
        let original_contents =
            song_ini_with_game_icon("Song", "song_checksum", "gamelogo_missing");
        write_test_file_contents(&song_ini_path, &original_contents);
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let error = apply_game_icon_song_fixes_paths(
            &settings,
            vec![GameIconSongFixInput {
                relative_path: "Song/song.ini".to_string(),
                new_game_icon: "gamelogo_unknown".to_string(),
            }],
            &store,
        )
        .expect_err("unknown replacement should fail");

        assert_eq!(
            error,
            "gamelogo_unknown is not a known official or custom GameIcon."
        );
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            original_contents
        );
    }

    #[test]
    fn game_icon_song_fix_apply_updates_game_icon_and_refreshes_scan_data() {
        let project = TestProject::new("game-icon-song-fix-apply");
        write_test_file_contents(
            &project
                .mods_dir
                .join("Icons")
                .join("gamelogo_known.img.xen"),
            "known",
        );
        let song_ini_path = project.mods_dir.join("Song").join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            &song_ini_with_game_icon("Song", "song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result = apply_game_icon_song_fixes_paths(
            &settings,
            vec![GameIconSongFixInput {
                relative_path: "Song/song.ini".to_string(),
                new_game_icon: "gamelogo_known".to_string(),
            }],
            &store,
        )
        .expect("known replacement should apply");

        assert_eq!(result.applied, 1);
        assert_eq!(result.songs[0].game_icon, "gamelogo_known");
        assert!(fs::read_to_string(song_ini_path)
            .expect("song.ini should read")
            .contains("GameIcon=gamelogo_known"));
    }

    #[test]
    fn game_icon_song_fix_apply_removes_empty_game_icon_entry() {
        let project = TestProject::new("game-icon-song-fix-remove");
        let song_ini_path = project.mods_dir.join("Song").join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            &song_ini_with_game_icon("Song", "song_checksum", "gamelogo_missing"),
        );
        let store = SongIniStore::default();
        let settings = test_scan_settings(&project);
        scan_song_ini_files_paths(&settings, &store).expect("scan should populate store");

        let result = apply_game_icon_song_fixes_paths(
            &settings,
            vec![GameIconSongFixInput {
                relative_path: "Song/song.ini".to_string(),
                new_game_icon: "".to_string(),
            }],
            &store,
        )
        .expect("empty replacement should remove GameIcon");

        assert_eq!(result.applied, 1);
        assert_eq!(result.songs[0].game_icon, "");
        assert!(!fs::read_to_string(song_ini_path)
            .expect("song.ini should read")
            .contains("GameIcon="));
    }

    fn write_test_file(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("test parent dir should be created");
        }
        fs::write(path, b"test").expect("test file should be written");
    }

    fn write_test_file_contents(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("test parent dir should be created");
        }
        fs::write(path, contents).expect("test file should be written");
    }

    fn write_valid_content_files(song_dir: &Path, checksum: &str) {
        let content_dir = song_dir.join("Content");
        let music_dir = content_dir.join("MUSIC");

        fs::create_dir_all(&music_dir).expect("content dirs should be created");
        write_test_file(&content_dir.join(format!("a{checksum}_song.pak.xen")));
        write_test_file(&music_dir.join(format!("{checksum}_preview.fsb.xen")));
        write_test_file(&music_dir.join(format!("{checksum}_1.fsb.xen")));
        write_test_file(&music_dir.join(format!("{checksum}_2.fsb.xen")));
        write_test_file(&music_dir.join(format!("{checksum}_3.fsb.xen")));
    }

    fn valid_song_ini(title: &str, checksum: &str) -> String {
        format!("[ModInfo]\nName={title}\n\n[SongInfo]\nChecksum={checksum}\nTitle={title}\n")
    }

    fn song_ini_with_game_icon(title: &str, checksum: &str, game_icon: &str) -> String {
        format!(
            "[ModInfo]\nName={title}\n\n[SongInfo]\nChecksum={checksum}\nTitle={title}\nGameIcon={game_icon}\n"
        )
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    configure_linux_settings_backend();

    tauri::Builder::default()
        .manage(SongIniStore::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings_path = app
                .path()
                .app_config_dir()
                .map_err(|err| format!("Failed to determine app settings directory: {err}"))?
                .join(SETTINGS_FILE_NAME);
            app.manage(SettingsStore {
                path: settings_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_project_settings,
            save_project_settings,
            third_party_licenses,
            preview_keep_only_files_delete,
            delete_keep_only_files,
            preview_ini_tool_categories,
            categorize_songs,
            scan_song_ini_files,
            validate_song_ini_file,
            undo_song_ini_repair,
            verify_content_issue_songs,
            disable_song_ini_file,
            enable_song_ini_file,
            delete_song_ini_conflict_file,
            set_scanned_song_included,
            update_scanned_song_metadata,
            restore_original_song_ini,
            restore_all_original_song_ini,
            scan_game_icon_categories,
            fix_game_icon_categories,
            preview_game_icon_song_fixes,
            apply_game_icon_song_fixes,
            analyze_scanned_song_instruments,
            analyze_song_pak
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "linux")]
fn configure_linux_settings_backend() {
    if env::var_os("GSETTINGS_BACKEND").is_none()
        && env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none()
    {
        env::set_var("GSETTINGS_BACKEND", "memory");
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_settings_backend() {}
