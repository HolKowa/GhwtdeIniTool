use ini::Ini;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
const DEFAULT_KEEP_ONLY_FILES_PATTERN: &str =
    "song.ini,*_song.pak.xen,*.fsb.xen,category.ini,*.img.xen";
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
    categories_extra_dir: Option<String>,
    categories_extra_dir_available: bool,
    keep_only_files_pattern: String,
    settings_file: String,
}

#[derive(Deserialize)]
struct ProjectSettingsInput {
    mods_dir: Option<String>,
    categories_extra_dir: Option<String>,
    keep_only_files_pattern: String,
}

#[derive(Default, Serialize)]
struct ScanModsResult {
    categories_found: usize,
    category_folders_moved: usize,
    files_moved: usize,
    renamed_destinations: usize,
    errors: Vec<String>,
    moved_categories_enabled: bool,
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
struct ScanModsPreview {
    categories_found: usize,
    files_to_move: Vec<String>,
    errors: Vec<String>,
    moved_categories_enabled: bool,
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
    faulty_files: Vec<FaultySongIniFile>,
    errors: Vec<String>,
}

#[derive(Clone, Serialize)]
struct FaultySongIniFile {
    relative_path: String,
    contents: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct SongIniValidationResult {
    relative_path: String,
    contents: String,
    songs_parsed: usize,
}

struct StoredProjectSettings {
    mods_dir: Option<String>,
    categories_extra_dir: Option<String>,
    keep_only_files_pattern: String,
}

#[tauri::command]
fn load_project_settings() -> Result<ProjectSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    Ok(project_settings(
        settings_path,
        create_missing_extra_dir(settings),
    ))
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
fn preview_scan_mods_folder() -> Result<ScanModsPreview, String> {
    let (mods_dir, categories_extra_dir) = scan_settings_paths()?;

    preview_scan_mods_folder_paths(&mods_dir, categories_extra_dir.as_deref())
}

#[tauri::command]
fn scan_mods_folder() -> Result<ScanModsResult, String> {
    let (mods_dir, categories_extra_dir) = scan_settings_paths()?;

    scan_mods_folder_paths(&mods_dir, categories_extra_dir.as_deref())
}

#[tauri::command]
fn preview_keep_only_files_delete() -> Result<DeleteFilesPreview, String> {
    let settings = scan_settings()?;
    preview_keep_only_files_delete_paths(&settings.mods_dir, &settings.keep_only_files_pattern)
}

#[tauri::command]
fn delete_keep_only_files(files_to_delete: Vec<String>) -> Result<DeleteFilesResult, String> {
    let settings = scan_settings()?;
    delete_keep_only_files_paths(&settings.mods_dir, &files_to_delete)
}

#[tauri::command]
fn scan_song_ini_files(store: tauri::State<'_, SongIniStore>) -> Result<SongIniScanResult, String> {
    let settings = scan_settings()?;
    scan_song_ini_files_paths(&settings.mods_dir, &store)
}

#[tauri::command]
fn validate_song_ini_file(
    relative_path: String,
    contents: String,
    store: tauri::State<'_, SongIniStore>,
) -> Result<SongIniValidationResult, String> {
    let settings = scan_settings()?;
    validate_song_ini_file_path(&settings.mods_dir, &relative_path, &contents, &store)
}

struct ScanSettings {
    mods_dir: PathBuf,
    categories_extra_dir: Option<PathBuf>,
    keep_only_files_pattern: String,
}

fn scan_settings_paths() -> Result<(PathBuf, Option<PathBuf>), String> {
    let settings = scan_settings()?;

    Ok((settings.mods_dir, settings.categories_extra_dir))
}

fn scan_settings() -> Result<ScanSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_project_settings_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => StoredProjectSettings::default(),
        Err(err) => return Err(settings_error(err)),
    };

    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;
    let categories_extra_dir = settings
        .categories_extra_dir
        .filter(|path| !path.trim().is_empty())
        .and_then(|path| {
            existing_or_created_extra_dir(path, &mods_dir, "Selected extra folder").ok()
        });

    Ok(ScanSettings {
        mods_dir,
        categories_extra_dir,
        keep_only_files_pattern: settings.keep_only_files_pattern,
    })
}

fn project_settings(settings_path: PathBuf, settings: StoredProjectSettings) -> ProjectSettings {
    let mods_dir_available = settings
        .mods_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());
    let categories_extra_dir_available = settings
        .categories_extra_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());

    ProjectSettings {
        mods_dir: settings.mods_dir,
        mods_dir_available,
        categories_extra_dir: settings.categories_extra_dir,
        categories_extra_dir_available,
        keep_only_files_pattern: settings.keep_only_files_pattern,
        settings_file: settings_path.to_string_lossy().into_owned(),
    }
}

fn create_missing_extra_dir(mut settings: StoredProjectSettings) -> StoredProjectSettings {
    let Some(mods_dir) = settings
        .mods_dir
        .as_ref()
        .and_then(|path| existing_dir(path.to_string(), "Selected MODS folder").ok())
    else {
        return settings;
    };
    let Some(categories_extra_dir) = settings
        .categories_extra_dir
        .as_ref()
        .filter(|path| !path.trim().is_empty())
    else {
        return settings;
    };

    if let Ok(path) = existing_or_created_extra_dir(
        categories_extra_dir.to_string(),
        &mods_dir,
        "Selected extra folder",
    ) {
        settings.categories_extra_dir = Some(path.to_string_lossy().into_owned());
    }

    settings
}

fn preview_scan_mods_folder_paths(
    mods_dir: &Path,
    categories_extra_dir: Option<&Path>,
) -> Result<ScanModsPreview, String> {
    let category_ini_paths = find_category_ini_files(mods_dir)?;
    let mut preview = ScanModsPreview {
        categories_found: category_ini_paths.len(),
        moved_categories_enabled: categories_extra_dir.is_some(),
        ..ScanModsPreview::default()
    };

    if categories_extra_dir.is_none() {
        return Ok(preview);
    }

    for category_ini_path in category_ini_paths {
        match category_data_files(&category_ini_path) {
            Ok(paths) => {
                preview.files_to_move.extend(
                    paths
                        .iter()
                        .map(|path| mods_relative_path(mods_dir, path))
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            Err(err) => preview.errors.push(err),
        }
    }

    Ok(preview)
}

fn scan_mods_folder_paths(
    mods_dir: &Path,
    categories_extra_dir: Option<&Path>,
) -> Result<ScanModsResult, String> {
    let category_ini_paths = find_category_ini_files(mods_dir)?;
    let mut result = ScanModsResult {
        categories_found: category_ini_paths.len(),
        moved_categories_enabled: categories_extra_dir.is_some(),
        ..ScanModsResult::default()
    };

    let Some(categories_extra_dir) = categories_extra_dir else {
        return Ok(result);
    };

    for category_ini_path in category_ini_paths {
        if let Err(err) = move_category_data(
            &category_ini_path,
            mods_dir,
            categories_extra_dir,
            &mut result,
        ) {
            result.errors.push(err);
        }
    }

    Ok(result)
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
    mods_dir: &Path,
    store: &SongIniStore,
) -> Result<SongIniScanResult, String> {
    let song_ini_paths = find_song_ini_files(mods_dir)?;
    let mut result = SongIniScanResult {
        songs_found: song_ini_paths.len(),
        ..SongIniScanResult::default()
    };
    let mut parsed_songs = Vec::new();

    for song_ini_path in song_ini_paths {
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
                    if let Err(err) = fs::write(&song_ini_path, &normalized_contents) {
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
    replace_song_ini_store(store, parsed_songs)?;
    Ok(result)
}

fn validate_song_ini_file_path(
    mods_dir: &Path,
    relative_path: &str,
    contents: &str,
    store: &SongIniStore,
) -> Result<SongIniValidationResult, String> {
    let path = checked_mods_relative_path(mods_dir, relative_path)?;

    if !is_song_ini(&path) {
        return Err(format!("{relative_path} is not a song.ini file."));
    }

    let normalized_contents = normalize_song_ini_key_case(contents);
    let parsed_song = parse_song_ini(relative_path, &normalized_contents)?;

    fs::write(&path, &normalized_contents)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    let songs_parsed = upsert_song_ini_store(store, parsed_song)?;

    Ok(SongIniValidationResult {
        relative_path: relative_path.to_string(),
        contents: normalized_contents,
        songs_parsed,
    })
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
) -> Result<usize, String> {
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

fn find_category_ini_files(mods_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut category_ini_paths = Vec::new();
    collect_category_ini_files(mods_dir, &mut category_ini_paths)?;
    Ok(category_ini_paths)
}

fn collect_category_ini_files(
    dir: &Path,
    category_ini_paths: &mut Vec<PathBuf>,
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
            collect_category_ini_files(&path, category_ini_paths)?;
        } else if file_type.is_file() && is_category_ini(&path) {
            category_ini_paths.push(path);
        }
    }

    Ok(())
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

fn move_category_data(
    category_ini_path: &Path,
    mods_dir: &Path,
    categories_extra_dir: &Path,
    result: &mut ScanModsResult,
) -> Result<(), String> {
    let source_dir = category_ini_path.parent().ok_or_else(|| {
        format!(
            "Could not determine category folder for {}.",
            category_ini_path.display()
        )
    })?;
    let folder_name = source_dir
        .file_name()
        .or_else(|| mods_dir.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("MODS");
    let (destination_dir, was_renamed) =
        next_available_destination(categories_extra_dir, folder_name);

    fs::create_dir(&destination_dir).map_err(|err| {
        format!(
            "Failed to create destination folder {}: {err}",
            destination_dir.display()
        )
    })?;

    let mut moved_files = 0;
    for source_path in category_data_files(category_ini_path)? {
        let Some(file_name) = source_path.file_name() else {
            result
                .errors
                .push(format!("Skipped unnamed file {}.", source_path.display()));
            continue;
        };
        let destination_path = destination_dir.join(file_name);

        match move_file(&source_path, &destination_path) {
            Ok(()) => moved_files += 1,
            Err(err) => result.errors.push(err),
        }
    }

    if moved_files == 0 {
        let _ = fs::remove_dir(&destination_dir);
        return Ok(());
    }

    result.category_folders_moved += 1;
    result.files_moved += moved_files;

    if was_renamed {
        result.renamed_destinations += 1;
    }

    Ok(())
}

fn category_data_files(category_ini_path: &Path) -> Result<Vec<PathBuf>, String> {
    let source_dir = category_ini_path.parent().ok_or_else(|| {
        format!(
            "Could not determine category folder for {}.",
            category_ini_path.display()
        )
    })?;
    let mut files = vec![category_ini_path.to_path_buf()];
    files.extend(sibling_img_xen_files(source_dir)?);

    Ok(files)
}

fn sibling_img_xen_files(source_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(source_dir).map_err(|err| {
        format!(
            "Failed to read category folder {}: {err}",
            source_dir.display()
        )
    })?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = entry
            .map_err(|err| format!("Failed to read entry in {}: {err}", source_dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;

        if file_type.is_file() && is_img_xen_file(&path) {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn next_available_destination(base_dir: &Path, folder_name: &str) -> (PathBuf, bool) {
    let destination = base_dir.join(folder_name);

    if !destination.exists() {
        return (destination, false);
    }

    for index in 1.. {
        let candidate = base_dir.join(format!("{folder_name} ({index})"));

        if !candidate.exists() {
            return (candidate, true);
        }
    }

    unreachable!("destination search should always find a free numbered folder")
}

fn move_file(source_path: &Path, destination_path: &Path) -> Result<(), String> {
    match fs::rename(source_path, destination_path) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            fs::copy(source_path, destination_path).map_err(|copy_err| {
                format!(
                    "Failed to move {} to {}: {rename_err}; copy fallback failed: {copy_err}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
            fs::remove_file(source_path).map_err(|remove_err| {
                format!(
                    "Copied {} to {}, but failed to remove the original file: {remove_err}",
                    source_path.display(),
                    destination_path.display()
                )
            })
        }
    }
}

fn is_category_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("category.ini"))
}

fn is_song_ini(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("song.ini"))
}

fn is_img_xen_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".img.xen"))
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
            "categories_extra_dir" => {
                settings.categories_extra_dir = (!value.is_empty()).then_some(value);
            }
            "keep_only_files_pattern" => settings.keep_only_files_pattern = value,
            _ => {}
        }
    }

    settings
}

fn write_project_settings_to_ini(settings: &StoredProjectSettings) -> String {
    format!(
        "[project]\nmods_dir={}\ncategories_extra_dir={}\nkeep_only_files_pattern={}\n",
        escape_ini_value(settings.mods_dir.as_deref().unwrap_or("")),
        escape_ini_value(settings.categories_extra_dir.as_deref().unwrap_or("")),
        escape_ini_value(&settings.keep_only_files_pattern)
    )
}

fn validate_project_settings(
    settings: ProjectSettingsInput,
) -> Result<StoredProjectSettings, String> {
    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;
    let categories_extra_dir = match settings.categories_extra_dir {
        Some(path) if !path.trim().is_empty() => {
            let categories_extra_dir =
                existing_or_created_extra_dir(path, &mods_dir, "Selected extra folder")?;

            Some(categories_extra_dir.to_string_lossy().into_owned())
        }
        _ => None,
    };

    Ok(StoredProjectSettings {
        mods_dir: Some(mods_dir.to_string_lossy().into_owned()),
        categories_extra_dir,
        keep_only_files_pattern: settings.keep_only_files_pattern,
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

fn existing_or_created_extra_dir(
    path: String,
    mods_dir: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    if path.exists() && !path.is_dir() {
        return Err(format!("{label} exists but is not a directory."));
    }

    if !path.exists() {
        reject_extra_dir_inside_mods(&path, mods_dir)?;
        fs::create_dir_all(&path)
            .map_err(|err| format!("Failed to create {label} at {}: {err}", path.display()))?;
    }

    let path = path
        .canonicalize()
        .map_err(|err| format!("Failed to resolve {label}: {err}"))?;

    if path == mods_dir || path.starts_with(mods_dir) {
        return Err("Selected extra folder cannot be the MODS folder or inside it.".to_string());
    }

    Ok(path)
}

fn reject_extra_dir_inside_mods(path: &Path, mods_dir: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if let Ok(parent) = parent.canonicalize() {
        if parent == mods_dir || parent.starts_with(mods_dir) {
            return Err(
                "Selected extra folder cannot be the MODS folder or inside it.".to_string(),
            );
        }
    }

    Ok(())
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

fn settings_error(err: io::Error) -> String {
    format!("Failed to locate settings file next to the executable: {err}")
}

impl Default for StoredProjectSettings {
    fn default() -> Self {
        Self {
            mods_dir: None,
            categories_extra_dir: None,
            keep_only_files_pattern: DEFAULT_KEEP_ONLY_FILES_PATTERN.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestProject {
        root: PathBuf,
        mods_dir: PathBuf,
        extra_dir: PathBuf,
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
            let extra_dir = root.join("Extra");

            fs::create_dir_all(&mods_dir).expect("mods dir should be created");
            fs::create_dir_all(&extra_dir).expect("extra dir should be created");

            Self {
                root,
                mods_dir,
                extra_dir,
            }
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn preview_lists_recursive_category_files_without_moving() {
        let project = TestProject::new("preview-category-data");
        let category_dir = project.mods_dir.join("Artist").join("Song");
        fs::create_dir_all(&category_dir).expect("category folder should be created");
        write_test_file(&category_dir.join("category.ini"));
        write_test_file(&category_dir.join("preview.IMG.XEN"));
        write_test_file(&category_dir.join("notes.txt"));

        let preview = preview_scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("preview should succeed");

        assert_eq!(preview.categories_found, 1);
        assert!(preview.moved_categories_enabled);
        assert_eq!(
            preview.files_to_move,
            vec![
                "Artist/Song/category.ini".to_string(),
                "Artist/Song/preview.IMG.XEN".to_string()
            ]
        );
        assert!(preview.errors.is_empty());
        assert!(category_dir.join("category.ini").exists());
        assert!(category_dir.join("preview.IMG.XEN").exists());
        assert!(!project.extra_dir.join("Song").exists());
    }

    #[test]
    fn preview_omits_mods_prefix_for_root_category_files() {
        let project = TestProject::new("preview-root-category");
        write_test_file(&project.mods_dir.join("category.ini"));

        let preview = preview_scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("preview should succeed");

        assert_eq!(preview.files_to_move, vec!["category.ini".to_string()]);
    }

    #[test]
    fn preview_is_empty_when_category_moving_is_disabled() {
        let project = TestProject::new("preview-scan-only");
        let category_dir = project.mods_dir.join("Category A");
        fs::create_dir_all(&category_dir).expect("category folder should be created");
        write_test_file(&category_dir.join("category.ini"));

        let preview = preview_scan_mods_folder_paths(&project.mods_dir, None)
            .expect("preview should succeed");

        assert_eq!(preview.categories_found, 1);
        assert!(!preview.moved_categories_enabled);
        assert!(preview.files_to_move.is_empty());
    }

    #[test]
    fn scan_only_counts_recursive_categories_without_moving() {
        let project = TestProject::new("scan-only");
        let nested_category = project.mods_dir.join("Artist").join("Song");
        fs::create_dir_all(&nested_category).expect("category folder should be created");
        write_test_file(&nested_category.join("category.ini"));

        let result = scan_mods_folder_paths(&project.mods_dir, None).expect("scan should succeed");

        assert_eq!(result.categories_found, 1);
        assert_eq!(result.category_folders_moved, 0);
        assert_eq!(result.files_moved, 0);
        assert_eq!(result.renamed_destinations, 0);
        assert!(!result.moved_categories_enabled);
        assert!(nested_category.join("category.ini").exists());
    }

    #[test]
    fn moves_category_ini_and_sibling_img_xen_files_only() {
        let project = TestProject::new("move-category-data");
        let category_dir = project.mods_dir.join("Category A");
        fs::create_dir_all(&category_dir).expect("category folder should be created");
        write_test_file(&category_dir.join("category.ini"));
        write_test_file(&category_dir.join("preview.IMG.XEN"));
        write_test_file(&category_dir.join("notes.txt"));

        let result = scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("scan should succeed");

        let destination_dir = project.extra_dir.join("Category A");
        assert_eq!(result.categories_found, 1);
        assert_eq!(result.category_folders_moved, 1);
        assert_eq!(result.files_moved, 2);
        assert_eq!(result.renamed_destinations, 0);
        assert!(result.moved_categories_enabled);
        assert!(result.errors.is_empty());
        assert!(destination_dir.join("category.ini").exists());
        assert!(destination_dir.join("preview.IMG.XEN").exists());
        assert!(!category_dir.join("category.ini").exists());
        assert!(!category_dir.join("preview.IMG.XEN").exists());
        assert!(category_dir.join("notes.txt").exists());
        assert!(category_dir.exists());
    }

    #[test]
    fn auto_renames_destination_folder_when_name_exists() {
        let project = TestProject::new("rename-conflict");
        let category_dir = project.mods_dir.join("Category A");
        fs::create_dir_all(&category_dir).expect("category folder should be created");
        fs::create_dir_all(project.extra_dir.join("Category A"))
            .expect("conflict folder should exist");
        write_test_file(&category_dir.join("category.ini"));

        let result = scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("scan should succeed");

        assert_eq!(result.category_folders_moved, 1);
        assert_eq!(result.files_moved, 1);
        assert_eq!(result.renamed_destinations, 1);
        assert!(project
            .extra_dir
            .join("Category A (1)")
            .join("category.ini")
            .exists());
    }

    #[test]
    fn category_ini_at_mods_root_uses_mods_folder_name() {
        let project = TestProject::new("root-category");
        write_test_file(&project.mods_dir.join("category.ini"));

        let result = scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("scan should succeed");

        assert_eq!(result.categories_found, 1);
        assert_eq!(result.category_folders_moved, 1);
        assert!(project.extra_dir.join("MODS").join("category.ini").exists());
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
    fn delete_preview_after_category_move_omits_moved_category_data() {
        let project = TestProject::new("delete-after-move");
        let category_dir = project.mods_dir.join("Category A");
        fs::create_dir_all(&category_dir).expect("category folder should be created");
        write_test_file(&category_dir.join("category.ini"));
        write_test_file(&category_dir.join("preview.img.xen"));
        write_test_file(&category_dir.join("notes.txt"));

        scan_mods_folder_paths(&project.mods_dir, Some(&project.extra_dir))
            .expect("scan should succeed");
        let preview = preview_keep_only_files_delete_paths(
            &project.mods_dir,
            DEFAULT_KEEP_ONLY_FILES_PATTERN,
        )
        .expect("preview should succeed");

        assert_eq!(
            preview.files_to_delete,
            vec!["Category A/notes.txt".to_string()]
        );
        assert!(project
            .extra_dir
            .join("Category A")
            .join("category.ini")
            .exists());
        assert!(project
            .extra_dir
            .join("Category A")
            .join("preview.img.xen")
            .exists());
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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
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
    fn song_scan_accepts_utf8_bom_before_modinfo() {
        let project = TestProject::new("song-scan-bom");
        write_test_file_contents(
            &project.mods_dir.join("song.ini"),
            "\u{feff}[ModInfo]\r\nName=With Bom\r\n\r\n[SongInfo]\r\nChecksum=with_bom\r\nTitle=With Bom\r\n",
        );
        let store = SongIniStore::default();

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
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
        write_test_file_contents(
            &song_ini_path,
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nartist=Motorhead\n",
        );
        let store = SongIniStore::default();

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.songs_found, 1);
        assert_eq!(result.songs_parsed, 1);
        assert!(result.faulty_files.is_empty());
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            "[ModInfo]\nName=Wrong Key Case\n\n[SongInfo]\nChecksum=wrong_key_case\nArtist=Motorhead\n"
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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");
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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");

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

        let result =
            scan_song_ini_files_paths(&project.mods_dir, &store).expect("scan should complete");

        assert_eq!(result.songs_parsed, 0);
        assert_eq!(result.faulty_files.len(), 1);
        assert!(result.faulty_files[0]
            .error
            .contains("Checksum entry in [SongInfo] must not be empty"));
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
            &project.mods_dir,
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
            &project.mods_dir,
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
            &project.mods_dir,
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
            &project.mods_dir,
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
        write_test_file_contents(
            &song_ini_path,
            valid_song_ini("Original", "original_checksum").as_str(),
        );
        let store = SongIniStore::default();

        let repaired_song_ini = valid_song_ini("Repaired", "repaired_checksum");
        let result =
            validate_song_ini_file_path(&project.mods_dir, "song.ini", &repaired_song_ini, &store)
                .expect("validation should pass");
        let stored_songs = store.0.lock().expect("store should lock");

        assert_eq!(result.relative_path, "song.ini");
        assert_eq!(result.songs_parsed, 1);
        assert_eq!(
            fs::read_to_string(song_ini_path).expect("song.ini should read"),
            repaired_song_ini
        );
        assert_eq!(stored_songs.len(), 1);
        assert_eq!(stored_songs[0].sections[1].entries[1].value, "Repaired");
    }

    #[test]
    fn song_validation_rejects_paths_outside_mods() {
        let project = TestProject::new("song-validate-outside");
        let outside_file = project.root.join("song.ini");
        write_test_file_contents(&outside_file, "[SongInfo]\nTitle=Outside\n");
        let store = SongIniStore::default();

        let result = validate_song_ini_file_path(
            &project.mods_dir,
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
    fn save_settings_creates_missing_extra_folder() {
        let project = TestProject::new("create-missing-extra");
        let missing_extra_dir = project.root.join("CreatedExtra");

        let settings = validate_project_settings(ProjectSettingsInput {
            mods_dir: Some(project.mods_dir.to_string_lossy().into_owned()),
            categories_extra_dir: Some(missing_extra_dir.to_string_lossy().into_owned()),
            keep_only_files_pattern: DEFAULT_KEEP_ONLY_FILES_PATTERN.to_string(),
        })
        .expect("settings should validate");

        assert!(missing_extra_dir.is_dir());
        assert_eq!(
            settings.categories_extra_dir,
            Some(
                missing_extra_dir
                    .canonicalize()
                    .expect("created extra dir should resolve")
                    .to_string_lossy()
                    .into_owned()
            )
        );
    }

    #[test]
    fn save_settings_rejects_missing_extra_folder_inside_mods_without_creating() {
        let project = TestProject::new("reject-missing-extra-inside-mods");
        let invalid_extra_dir = project.mods_dir.join("Extra");

        let result = validate_project_settings(ProjectSettingsInput {
            mods_dir: Some(project.mods_dir.to_string_lossy().into_owned()),
            categories_extra_dir: Some(invalid_extra_dir.to_string_lossy().into_owned()),
            keep_only_files_pattern: DEFAULT_KEEP_ONLY_FILES_PATTERN.to_string(),
        });
        let err = match result {
            Ok(_) => panic!("settings should reject extra dir inside MODS"),
            Err(err) => err,
        };

        assert_eq!(
            err,
            "Selected extra folder cannot be the MODS folder or inside it."
        );
        assert!(!invalid_extra_dir.exists());
    }

    fn write_test_file(path: &Path) {
        fs::write(path, b"test").expect("test file should be written");
    }

    fn write_test_file_contents(path: &Path, contents: &str) {
        fs::write(path, contents).expect("test file should be written");
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
            preview_scan_mods_folder,
            scan_mods_folder,
            preview_keep_only_files_delete,
            delete_keep_only_files,
            scan_song_ini_files,
            validate_song_ini_file
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
