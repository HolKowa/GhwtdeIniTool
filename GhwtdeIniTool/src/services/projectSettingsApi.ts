import { invoke } from "@tauri-apps/api/core";

import type { ProjectSettings } from "../types/projectSettings";

export function loadProjectSettings() {
  return invoke<ProjectSettings>("load_project_settings");
}

export function saveProjectSettings(settings: ProjectSettings) {
  return invoke<ProjectSettings>("save_project_settings", {
    settings: {
      disclaimer_accepted: settings.disclaimer_accepted,
      mods_dir: settings.mods_dir,
      official_gamelogos_dir: settings.official_gamelogos_dir,
      keep_original_song_ini: settings.keep_original_song_ini,
      check_for_updates_on_startup: settings.check_for_updates_on_startup,
    },
  });
}
