import { invoke } from "@tauri-apps/api/core";

import type { ProjectSettings } from "../types/projectSettings";

export function loadProjectSettings() {
  return invoke<ProjectSettings>("load_project_settings");
}

export function saveProjectSettings(settings: ProjectSettings) {
  return invoke<ProjectSettings>("save_project_settings", {
    settings: {
      mods_dir: settings.mods_dir,
      categories_extra_dir: settings.categories_extra_dir,
      keep_only_files_with_pattern: settings.keep_only_files_with_pattern,
      keep_only_files_pattern: settings.keep_only_files_pattern,
    },
  });
}
