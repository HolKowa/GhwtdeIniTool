use ini::Ini;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

mod song_pak_analyzer;

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
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
struct SongIniDeleteResult {
    relative_path: String,
    songs_parsed: usize,
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
fn scan_song_ini_files(store: tauri::State<'_, SongIniStore>) -> Result<SongIniScanResult, String> {
    let settings = scan_settings()?;
    scan_song_ini_files_paths(&settings, &store)
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
fn disable_song_ini_file(
    relative_path: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniDisableResult, String> {
    let settings = scan_settings()?;
    disable_song_ini_file_path(&settings, &relative_path, &store)
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

fn scan_song_ini_files_paths(
    settings: &ScanSettings,
    store: &SongIniStore,
) -> Result<SongIniScanResult, String> {
    let mods_dir = &settings.mods_dir;
    let song_ini_paths = find_song_ini_files(mods_dir)?;
    let mut result = SongIniScanResult {
        songs_found: song_ini_paths.len(),
        ..SongIniScanResult::default()
    };
    let mut parsed_songs = Vec::new();

    for song_ini_path in &song_ini_paths {
        let relative_path = mods_relative_path(mods_dir, &song_ini_path)?;
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
    result.content_file_issues = song_content_issues(mods_dir, &parsed_songs)?;
    replace_song_ini_store(store, parsed_songs)?;
    Ok(result)
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
    let backup_path = path.with_file_name("song.original.ini");

    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(path, backup_path).map(|_| ())
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

    ScannedSong {
        relative_path: song.relative_path.clone(),
        folder_absolute_path,
        artist: song_info_value(song, "Artist"),
        title: song_info_value(song, "Title"),
        year: song_info_value(song, "Year"),
        genre: song_info_value(song, "Genre"),
        game_icon: song_info_value(song, "GameIcon"),
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
    let mut issues = Vec::new();

    for song in parsed_songs {
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
            disable_song_ini_file,
            delete_song_ini_conflict_file,
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
