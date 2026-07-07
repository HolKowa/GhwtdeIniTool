use ini::Ini;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, UNIX_EPOCH},
};
use tauri::Emitter;

mod song_pak_analyzer;

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
const INSTRUMENT_SIDECAR_FILE_NAME: &str = "song.instruments.ini";
const INSTRUMENT_ANALYZER_VERSION: &str = "1";
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
    mods_dir: Option<String>,
    mods_dir_available: bool,
    keep_original_song_ini: bool,
    settings_file: String,
}

#[derive(Deserialize)]
struct ProjectSettingsInput {
    mods_dir: Option<String>,
    keep_original_song_ini: Option<bool>,
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

#[derive(Default)]
struct SongIniStore(Mutex<Vec<ParsedSongIni>>);

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
    disabled_song_conflicts: Vec<DisabledSongConflict>,
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
struct DisabledSongConflict {
    active_path: String,
    disabled_path: String,
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
    disabled_song_conflicts: Vec<DisabledSongConflict>,
    content_file_issues: Vec<SongContentIssue>,
}

#[derive(Debug, Serialize)]
struct SongIniDisableResult {
    relative_path: String,
    disabled_path: String,
    songs_parsed: usize,
}

#[derive(Debug, Serialize)]
struct SongIniEnableResult {
    relative_path: String,
    enabled_path: String,
    songs_parsed: usize,
}

#[derive(Debug, Serialize)]
struct SongIniDeleteResult {
    relative_path: String,
    songs_parsed: usize,
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
struct SongScanProgress {
    phase: String,
    current: usize,
    total: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    relative_path: String,
}

struct StoredProjectSettings {
    mods_dir: Option<String>,
    keep_original_song_ini: bool,
}

#[tauri::command]
fn load_project_settings() -> Result<ProjectSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    Ok(project_settings(settings_path, settings))
}

#[tauri::command]
fn save_project_settings(settings: ProjectSettingsInput) -> Result<ProjectSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let settings = validate_project_settings(settings)?;
    let contents = write_project_settings_to_ini(&settings);

    fs::write(&settings_path, contents).map_err(|err| {
        format!(
            "Failed to write settings file at {}: {err}",
            settings_path.display()
        )
    })?;

    Ok(project_settings(settings_path, settings))
}

#[tauri::command]
fn preview_keep_only_files_delete(
    keep_only_files_pattern: String,
) -> Result<DeleteFilesPreview, String> {
    let settings = scan_settings()?;
    preview_keep_only_files_delete_paths(&settings.mods_dir, &keep_only_files_pattern)
}

#[tauri::command]
fn delete_keep_only_files(files_to_delete: Vec<String>) -> Result<DeleteFilesResult, String> {
    let settings = scan_settings()?;
    delete_keep_only_files_paths(&settings.mods_dir, &files_to_delete)
}

#[tauri::command]
fn scan_song_ini_files(
    app: tauri::AppHandle,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniScanResult, String> {
    let settings = scan_settings()?;
    scan_song_ini_files_paths_with_progress(&settings, &store, |progress| {
        let _ = app.emit("song_scan_progress", progress);
    })
}

#[tauri::command]
fn validate_song_ini_file(
    relative_path: String,
    contents: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings()?;
    validate_song_ini_file_path(&settings, &relative_path, &contents, &store)
}

#[tauri::command]
fn verify_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings()?;
    verify_song_ini_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn disable_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniDisableResult, String> {
    let settings = scan_settings()?;
    disable_song_ini_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn enable_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniEnableResult, String> {
    let settings = scan_settings()?;
    enable_song_ini_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn delete_song_ini_conflict_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniDeleteResult, String> {
    let settings = scan_settings()?;
    delete_song_ini_conflict_file_path(&settings, &relative_path, &store)
}

#[tauri::command]
fn update_scanned_song_metadata(
    relative_path: String,
    metadata: ScannedSongMetadataInput,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings()?;
    update_scanned_song_metadata_path(&settings, &relative_path, metadata, &store)
}

#[tauri::command]
fn restore_original_song_ini(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings()?;
    restore_original_song_ini_path(&settings, &relative_path, &store)
}

#[tauri::command]
async fn analyze_scanned_song_instruments(
    app: tauri::AppHandle,
    mode: InstrumentAnalyzeMode,
    store: tauri::State<'_, SongIniStore>,
) -> Result<InstrumentAnalyzeResult, String> {
    let settings = scan_settings()?;
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
    keep_original_song_ini: bool,
}

fn scan_settings() -> Result<ScanSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;

    Ok(ScanSettings {
        mods_dir,
        keep_original_song_ini: settings.keep_original_song_ini,
    })
}

fn project_settings(settings_path: PathBuf, settings: StoredProjectSettings) -> ProjectSettings {
    let mods_dir_available = settings
        .mods_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());

    ProjectSettings {
        mods_dir: settings.mods_dir,
        mods_dir_available,
        keep_original_song_ini: settings.keep_original_song_ini,
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

fn scan_song_ini_files_paths_with_progress<F>(
    settings: &ScanSettings,
    store: &SongIniStore,
    mut emit_progress: F,
) -> Result<SongIniScanResult, String>
where
    F: FnMut(SongScanProgress),
{
    let mods_dir = &settings.mods_dir;
    emit_progress(song_scan_progress("findingSongs", 0, 0, ""));
    let song_ini_paths = find_song_ini_files(mods_dir)?;
    let mut result = SongIniScanResult {
        songs_found: song_ini_paths.len(),
        ..SongIniScanResult::default()
    };
    let mut parsed_songs = Vec::new();

    for (index, song_ini_path) in song_ini_paths.iter().enumerate() {
        let relative_path = mods_relative_path(mods_dir, &song_ini_path)?;
        emit_progress(song_scan_progress(
            "readingSongs",
            index + 1,
            song_ini_paths.len(),
            &relative_path,
        ));
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
    result.disabled_song_conflicts = disabled_song_conflicts(mods_dir, &song_ini_paths)?;
    result.content_file_issues = song_content_issues_with_progress(
        mods_dir,
        &parsed_songs,
        |current, total, relative_path| {
            emit_progress(song_scan_progress(
                "checkingContent",
                current,
                total,
                relative_path,
            ));
        },
    )?;
    emit_progress(song_scan_progress("finishing", 0, 0, ""));
    replace_song_ini_store(store, parsed_songs)?;
    Ok(result)
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

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

    let normalized_contents = normalize_song_ini_key_case(contents);
    let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;

    write_song_ini_file(&path, &normalized_contents, settings.keep_original_song_ini)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    let parsed_songs = upsert_song_ini_store(store, parsed_song)?;
    let songs_parsed = parsed_songs.len();
    let song_ini_paths = find_song_ini_files(mods_dir)?;

    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents: normalized_contents,
        songs_parsed,
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        disabled_song_conflicts: disabled_song_conflicts(mods_dir, &song_ini_paths)?,
        content_file_issues: song_content_issues(mods_dir, &parsed_songs)?,
    })
}

fn verify_song_ini_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let normalized_contents = normalize_song_ini_key_case(&contents);
    let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;

    refreshed_song_ini_result(
        settings,
        relative_path,
        normalized_contents,
        parsed_song,
        store,
    )
}

fn disable_song_ini_file_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniDisableResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
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

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

    if path.exists() {
        return Err(format!("{relative_path} is already enabled."));
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
    let parsed_song =
        parse_song_ini(relative_path, &normalize_song_ini_key_case(&disabled_contents)).ok();

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

    if !is_song_ini(&path) && !is_disabled_song_ini(&path) {
        return Err(format!(
            "{relative_path} is not a song.ini or song.disabled.ini file."
        ));
    }

    fs::remove_file(&path).map_err(|err| format!("Failed to delete {}: {err}", path.display()))?;

    let songs_parsed = if is_song_ini(&path) {
        remove_song_ini_from_store(store, relative_path)?
    } else {
        store_song_count(store)?
    };

    Ok(SongIniDeleteResult {
        relative_path: relative_path.to_string(),
        songs_parsed,
    })
}

fn update_scanned_song_metadata_path(
    settings: &ScanSettings,
    relative_path: &str,
    metadata: ScannedSongMetadataInput,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let normalized_contents = normalize_song_ini_key_case(&contents);
    parse_song_ini(relative_path, &normalized_contents)?;
    let updated_contents = update_song_ini_metadata_contents(&normalized_contents, &metadata);
    let parsed_song = parse_song_ini(relative_path, &updated_contents)?;

    write_song_ini_file(&path, &updated_contents, settings.keep_original_song_ini)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    refreshed_song_ini_result(
        settings,
        relative_path,
        updated_contents,
        parsed_song,
        store,
    )
}

fn restore_original_song_ini_path(
    settings: &ScanSettings,
    relative_path: &str,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let mods_dir = &settings.mods_dir;
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

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
        relative_path,
        &normalize_song_ini_key_case(&backup_contents),
    )?;

    fs::copy(&backup_path, &path).map_err(|err| {
        format!(
            "Failed to restore {} from {}: {err}",
            path.display(),
            backup_path.display()
        )
    })?;
    fs::remove_file(&backup_path)
        .map_err(|err| format!("Failed to remove {}: {err}", backup_path.display()))?;

    refreshed_song_ini_result(settings, relative_path, backup_contents, parsed_song, store)
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
    let song_ini_paths = find_song_ini_files(mods_dir)?;

    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents,
        songs_parsed,
        songs: scanned_songs(mods_dir, &parsed_songs),
        duplicate_checksum_groups: duplicate_checksum_groups(&parsed_songs),
        disabled_song_conflicts: disabled_song_conflicts(mods_dir, &song_ini_paths)?,
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

fn backup_original_song_ini(path: &Path) -> io::Result<()> {
    let backup_path = original_song_ini_path(path);

    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(path, backup_path).map(|_| ())
}

fn original_song_ini_path(path: &Path) -> PathBuf {
    path.with_file_name("song.original.ini")
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
    let mut updated_contents = String::with_capacity(contents.len());
    let mut in_song_info = false;
    let mut found_keys: HashSet<&str> = HashSet::new();
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
                    let leading_whitespace_len = key_part.len() - key_part.trim_start().len();
                    let trailing_whitespace_len = key_part.len() - key_part.trim_end().len();
                    updated_contents.push_str(&key_part[..leading_whitespace_len]);
                    updated_contents.push_str(canonical_key);
                    updated_contents
                        .push_str(&key_part[key_part.len() - trailing_whitespace_len..]);
                    updated_contents.push('=');
                    updated_contents.push_str(value);
                    updated_contents.push_str(line_ending);
                    found_keys.insert(*canonical_key);
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

fn append_missing_metadata_keys(
    contents: &mut String,
    updates: &[(&'static str, &str)],
    found_keys: &HashSet<&str>,
    line_ending: &str,
) {
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push_str(line_ending);
    }

    for (key, value) in updates {
        if found_keys.contains(key) {
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

fn disabled_song_conflicts(
    mods_dir: &Path,
    song_ini_paths: &[PathBuf],
) -> Result<Vec<DisabledSongConflict>, String> {
    let mut conflicts = Vec::new();

    for song_ini_path in song_ini_paths {
        let Some(disabled_path) = find_sibling_disabled_song_ini(song_ini_path)? else {
            continue;
        };

        conflicts.push(DisabledSongConflict {
            active_path: mods_relative_path(mods_dir, song_ini_path)?,
            disabled_path: mods_relative_path(mods_dir, &disabled_path)?,
        });
    }

    conflicts.sort_by(|left, right| left.active_path.cmp(&right.active_path));
    Ok(conflicts)
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
    let mut songs = store
        .0
        .lock()
        .map_err(|_| "Failed to lock song.ini store.".to_string())?;

    songs.retain(|song| song.relative_path != relative_path);
    Ok(songs.len())
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
        .map_err(|err| format!("Failed to resolve {}: {err}", parent_path.display()))?;

    if !canonical_parent_path.starts_with(mods_dir) {
        return Err(format!("Rejected path outside MODS: {relative_path}."));
    }

    let Some(file_name) = full_path.file_name() else {
        return Err(format!("Rejected unsafe MODS-relative path {relative_path}."));
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

fn is_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.ini"))
}

fn is_disabled_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.disabled.ini"))
}

fn settings_file_path() -> io::Result<PathBuf> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine executable directory.",
        )
    })?;

    Ok(exe_dir.join(SETTINGS_FILE_NAME))
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
            "mods_dir" => settings.mods_dir = (!value.is_empty()).then_some(value),
            "keep_original_song_ini" => {
                settings.keep_original_song_ini = parse_ini_bool(&value).unwrap_or(true)
            }
            _ => {}
        }
    }

    settings
}

fn write_project_settings_to_ini(settings: &StoredProjectSettings) -> String {
    format!(
        "[project]\nmods_dir={}\nkeep_original_song_ini={}\n",
        escape_ini_value(settings.mods_dir.as_deref().unwrap_or("")),
        settings.keep_original_song_ini
    )
}

fn validate_project_settings(
    settings: ProjectSettingsInput,
) -> Result<StoredProjectSettings, String> {
    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;

    Ok(StoredProjectSettings {
        mods_dir: Some(mods_dir.to_string_lossy().into_owned()),
        keep_original_song_ini: settings.keep_original_song_ini.unwrap_or(true),
    })
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
        .map_err(|err| format!("Failed to resolve {label}: {err}"))
}

fn escape_ini_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_ini_value(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();

    while let Some(char) = chars.next() {
        if char == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
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

fn parse_ini_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn settings_error(err: io::Error) -> String {
    format!("Failed to locate settings file next to the executable: {err}")
}

impl Default for StoredProjectSettings {
    fn default() -> Self {
        Self {
            mods_dir: None,
            keep_original_song_ini: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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
            keep_original_song_ini: true,
        }
    }

    fn test_scan_settings_without_song_ini_backup(project: &TestProject) -> ScanSettings {
        ScanSettings {
            mods_dir: project.mods_dir.clone(),
            keep_original_song_ini: false,
        }
    }

    #[test]
    fn settings_default_to_keeping_original_song_ini() {
        let settings = read_project_settings_from_ini("[project]\nmods_dir=/tmp/MODS\n");

        assert_eq!(settings.mods_dir.as_deref(), Some("/tmp/MODS"));
        assert!(settings.keep_original_song_ini);
    }

    #[test]
    fn settings_parse_keep_original_song_ini() {
        let settings = read_project_settings_from_ini(
            "[project]\nmods_dir=/tmp/MODS\nkeep_original_song_ini=false\n",
        );

        assert!(!settings.keep_original_song_ini);
    }

    #[test]
    fn settings_writer_includes_keep_original_song_ini() {
        let settings = StoredProjectSettings {
            mods_dir: Some("/tmp/MODS".to_string()),
            keep_original_song_ini: false,
        };

        assert_eq!(
            write_project_settings_to_ini(&settings),
            "[project]\nmods_dir=/tmp/MODS\nkeep_original_song_ini=false\n"
        );
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
        assert_eq!(reading_events.len(), 2);
        assert_eq!(reading_events[0].current, 1);
        assert_eq!(reading_events[0].total, 2);
        assert_eq!(reading_events[1].current, 2);
        assert_eq!(reading_events[1].total, 2);
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
        assert_eq!(content_events.len(), 2);
        assert_eq!(content_events[0].current, 1);
        assert_eq!(content_events[0].total, 2);
        assert_eq!(content_events[1].current, 2);
        assert_eq!(content_events[1].total, 2);
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
        assert_eq!(result.disabled_song_conflicts.len(), 1);
        assert_eq!(result.disabled_song_conflicts[0].active_path, "song.ini");
        assert_eq!(
            result.disabled_song_conflicts[0].disabled_path,
            "song.disabled.ini"
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
    fn song_validation_writes_valid_contents_and_updates_store() {
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
        assert_eq!(
            fs::read_to_string(project.mods_dir.join("song.original.ini"))
                .expect("song.original.ini should read"),
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
        assert_eq!(scan_result.disabled_song_conflicts.len(), 1);

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
        assert_eq!(validation_result.disabled_song_conflicts.len(), 1);
        assert_eq!(
            validation_result.disabled_song_conflicts[0].active_path,
            "Repaired/song.ini"
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
    fn song_verify_refreshes_content_issues_without_rewriting_song_ini() {
        let project = TestProject::new("song-verify-refresh-content");
        let song_ini_path = project.mods_dir.join("song.ini");
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "verify_checksum").as_str(),
        );
        write_valid_content_files(&project.mods_dir, "verify_checksum");
        fs::remove_file(
            project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("verify_checksum_3.fsb.xen"),
        )
        .expect("required file should be removed");
        let store = SongIniStore::default();

        let scan_result = scan_song_ini_files_paths(&test_scan_settings(&project), &store)
            .expect("scan should complete");
        assert!(scan_result.content_file_issues.iter().any(|issue| {
            issue.song_ini_relative_path == "song.ini" && issue.message == "Missing required file."
        }));

        write_test_file(
            &project
                .mods_dir
                .join("Content")
                .join("MUSIC")
                .join("verify_checksum_3.fsb.xen"),
        );

        let result = verify_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("verify should pass");

        assert!(result
            .content_file_issues
            .iter()
            .all(|issue| issue.song_ini_relative_path != "song.ini"));
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            valid_song_ini("Original", "verify_checksum")
        );
    }

    #[test]
    fn song_verify_rejects_invalid_or_unsafe_song_ini_paths() {
        let project = TestProject::new("song-verify-rejects");
        write_test_file_contents(&project.mods_dir.join("notes.txt"), "not a song");
        let store = SongIniStore::default();

        assert_eq!(
            verify_song_ini_file_path(&test_scan_settings(&project), "../song.ini", &store)
                .expect_err("unsafe path should be rejected"),
            "Rejected unsafe MODS-relative path ../song.ini."
        );
        assert_eq!(
            verify_song_ini_file_path(&test_scan_settings(&project), "notes.txt", &store)
                .expect_err("non-song.ini should be rejected"),
            "notes.txt is not a song.ini file."
        );
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
            restore_original_song_ini_path(&test_scan_settings(&project), "song.ini", &store)
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
    fn song_restore_original_rejects_missing_backup() {
        let project = TestProject::new("song-restore-missing-original");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            valid_song_ini("Current", "current_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let result =
            restore_original_song_ini_path(&test_scan_settings(&project), "song.ini", &store)
                .expect_err("restore without backup should fail");

        assert_eq!(
            result,
            "song.ini has no original song.ini backup to restore."
        );
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
        write_test_file_contents(&project.mods_dir.join("song.disabled.ini"), &disabled_song_ini);
        let store = SongIniStore::default();

        let result = enable_song_ini_file_path(&test_scan_settings(&project), "song.ini", &store)
            .expect("enable should succeed");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.enabled_path, "song.ini");
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
        write_test_file_contents(&project.mods_dir.join("song.disabled.ini"), disabled_song_ini);
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
            "notes.txt is not a song.ini or song.disabled.ini file."
        );
        assert!(outside_file.exists());
        assert!(project.mods_dir.join("notes.txt").exists());
    }

    fn write_test_file(path: &Path) {
        fs::write(path, b"test").expect("test file should be written");
    }

    fn write_test_file_contents(path: &Path, contents: &str) {
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
        .invoke_handler(tauri::generate_handler![
            load_project_settings,
            save_project_settings,
            preview_keep_only_files_delete,
            delete_keep_only_files,
            scan_song_ini_files,
            validate_song_ini_file,
            verify_song_ini_file,
            disable_song_ini_file,
            enable_song_ini_file,
            delete_song_ini_conflict_file,
            update_scanned_song_metadata,
            restore_original_song_ini,
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
