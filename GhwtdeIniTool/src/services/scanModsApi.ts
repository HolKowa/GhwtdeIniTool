import { invoke } from "@tauri-apps/api/core";

import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  ScanModsPreview,
  ScanModsResult,
  SongIniScanResult,
  SongIniValidationResult,
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

export function scanSongIniFiles() {
  return invoke<SongIniScanResult>("scan_song_ini_files");
}

export function validateSongIniFile(relativePath: string, contents: string) {
  return invoke<SongIniValidationResult>("validate_song_ini_file", {
    relativePath,
    contents,
  });
}
