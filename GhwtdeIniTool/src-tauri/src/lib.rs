use serde::Serialize;
use std::{env, fs, io, path::PathBuf};

const SETTINGS_FILE_NAME: &str = "ghwtdeinitool.ini";

#[derive(Serialize)]
struct ProjectSettings {
    mods_dir: Option<String>,
    mods_dir_available: bool,
    settings_file: String,
}

#[tauri::command]
fn load_project_settings() -> Result<ProjectSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let mods_dir = match fs::read_to_string(&settings_path) {
        Ok(contents) => read_mods_dir_from_ini(&contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(settings_error(err)),
    };

    Ok(project_settings(settings_path, mods_dir))
}

#[tauri::command]
fn save_project_settings(mods_dir: String) -> Result<ProjectSettings, String> {
    let settings_path = settings_file_path().map_err(settings_error)?;
    let mods_dir = PathBuf::from(mods_dir);

    if !mods_dir.is_dir() {
        return Err("Selected MODS folder does not exist or is not a directory.".to_string());
    }

    let mods_dir = mods_dir
        .canonicalize()
        .map_err(|err| format!("Failed to resolve selected MODS folder: {err}"))?;
    let mods_dir = mods_dir.to_string_lossy().into_owned();
    let contents = format!("[project]\nmods_dir={}\n", escape_ini_value(&mods_dir));

    fs::write(&settings_path, contents).map_err(|err| {
        format!(
            "Failed to write settings file at {}: {err}",
            settings_path.display()
        )
    })?;

    Ok(project_settings(settings_path, Some(mods_dir)))
}

fn project_settings(settings_path: PathBuf, mods_dir: Option<String>) -> ProjectSettings {
    let mods_dir_available = mods_dir
        .as_ref()
        .is_some_and(|path| PathBuf::from(path).is_dir());

    ProjectSettings {
        mods_dir,
        mods_dir_available,
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

fn read_mods_dir_from_ini(contents: &str) -> Option<String> {
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

        if in_project_section {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            if key.trim() == "mods_dir" {
                let value = unescape_ini_value(value.trim());
                return (!value.is_empty()).then_some(value);
            }
        }
    }

    None
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
