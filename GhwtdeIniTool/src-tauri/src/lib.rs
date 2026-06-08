use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
const DEFAULT_KEEP_ONLY_FILES_PATTERN: &str =
    "song.ini,*_song.pak.xen,*.fsb.xen,category.ini,*.img.xen";

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
struct ScanModsPreview {
    categories_found: usize,
    files_to_move: Vec<String>,
    errors: Vec<String>,
    moved_categories_enabled: bool,
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

fn scan_settings_paths() -> Result<(PathBuf, Option<PathBuf>), String> {
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

    Ok((mods_dir, categories_extra_dir))
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    configure_linux_settings_backend();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_project_settings,
            save_project_settings,
            preview_scan_mods_folder,
            scan_mods_folder
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
