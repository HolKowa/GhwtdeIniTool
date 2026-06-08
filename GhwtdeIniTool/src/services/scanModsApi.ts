import { invoke } from "@tauri-apps/api/core";

import type { ScanModsResult } from "../types/scanMods";

export function scanModsFolder() {
  return invoke<ScanModsResult>("scan_mods_folder");
}
