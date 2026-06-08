import { invoke } from "@tauri-apps/api/core";

import type { ScanModsPreview, ScanModsResult } from "../types/scanMods";

export function previewScanModsFolder() {
  return invoke<ScanModsPreview>("preview_scan_mods_folder");
}

export function scanModsFolder() {
  return invoke<ScanModsResult>("scan_mods_folder");
}
