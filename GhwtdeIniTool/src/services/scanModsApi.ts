import { invoke } from "@tauri-apps/api/core";

import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  ScanModsPreview,
  ScanModsResult,
} from "../types/scanMods";

export function previewScanModsFolder() {
  return invoke<ScanModsPreview>("preview_scan_mods_folder");
}

export function scanModsFolder() {
  return invoke<ScanModsResult>("scan_mods_folder");
}

export function previewKeepOnlyFilesDelete() {
  return invoke<DeleteFilesPreview>("preview_keep_only_files_delete");
}

export function deleteKeepOnlyFiles(filesToDelete: string[]) {
  return invoke<DeleteFilesResult>("delete_keep_only_files", {
    filesToDelete,
  });
}
