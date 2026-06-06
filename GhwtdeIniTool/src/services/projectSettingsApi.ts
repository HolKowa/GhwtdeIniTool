import { invoke } from "@tauri-apps/api/core";

import type { ProjectSettings } from "../types/projectSettings";

export function loadProjectSettings() {
  return invoke<ProjectSettings>("load_project_settings");
}

export function saveProjectSettings(modsDir: string) {
  return invoke<ProjectSettings>("save_project_settings", {
    modsDir,
  });
}
