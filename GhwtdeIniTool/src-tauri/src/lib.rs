use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";
const DEFAULT_KEEP_ONLY_FILES_PATTERN: &str = "song.ini,*_song.pak.xen,*.fsb.xen";

#[derive(Serialize)]
struct ProjectSettings {
    mods_dir: Option<String>,
    mods_dir_available: bool,
    categories_extra_dir: Option<String>,
    categories_extra_dir_available: bool,
    keep_only_files_with_pattern: bool,
    keep_only_files_pattern: String,
    settings_file: String,
}

#[derive(Deserialize)]
struct ProjectSettingsInput {
    mods_dir: Option<String>,
    categories_extra_dir: Option<String>,
    keep_only_files_with_pattern: bool,
    keep_only_files_pattern: String,
}

struct StoredProjectSettings {
    mods_dir: Option<String>,
    categories_extra_dir: Option<String>,
    keep_only_files_with_pattern: bool,
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
        keep_only_files_with_pattern: settings.keep_only_files_with_pattern,
        keep_only_files_pattern: settings.keep_only_files_pattern,
        settings_file: settings_path.to_string_lossy().into_owned(),
    }
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
            "keep_only_files_with_pattern" => {
                settings.keep_only_files_with_pattern = value.eq_ignore_ascii_case("true");
            }
            "keep_only_files_pattern" => settings.keep_only_files_pattern = value,
            _ => {}
        }
    }

    settings
}

fn write_project_settings_to_ini(settings: &StoredProjectSettings) -> String {
    format!(
        "[project]\nmods_dir={}\ncategories_extra_dir={}\nkeep_only_files_with_pattern={}\nkeep_only_files_pattern={}\n",
        escape_ini_value(settings.mods_dir.as_deref().unwrap_or("")),
        escape_ini_value(settings.categories_extra_dir.as_deref().unwrap_or("")),
        settings.keep_only_files_with_pattern,
        escape_ini_value(&settings.keep_only_files_pattern)
    )
}

fn validate_project_settings(
    settings: ProjectSettingsInput,
) -> Result<StoredProjectSettings, String> {
    let mods_dir = required_existing_dir(settings.mods_dir, "Selected MODS folder")?;
    let categories_extra_dir = match settings.categories_extra_dir {
        Some(path) if !path.trim().is_empty() => {
            let categories_extra_dir = existing_dir(path, "Selected extra folder")?;

            if categories_extra_dir == mods_dir || categories_extra_dir.starts_with(&mods_dir) {
                return Err(
                    "Selected extra folder cannot be the MODS folder or inside it.".to_string(),
                );
            }

            Some(categories_extra_dir.to_string_lossy().into_owned())
        }
        _ => None,
    };

    Ok(StoredProjectSettings {
        mods_dir: Some(mods_dir.to_string_lossy().into_owned()),
        categories_extra_dir,
        keep_only_files_with_pattern: settings.keep_only_files_with_pattern,
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
            keep_only_files_with_pattern: false,
            keep_only_files_pattern: DEFAULT_KEEP_ONLY_FILES_PATTERN.to_string(),
        }
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
            save_project_settings
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
