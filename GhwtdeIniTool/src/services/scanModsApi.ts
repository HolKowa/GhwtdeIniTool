import { invoke } from "@tauri-apps/api/core";

import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  SongIniDeleteResult,
  SongIniDisableResult,
  SongIniScanResult,
  SongIniValidationResult,
} from "../types/scanMods";

export function previewKeepOnlyFilesDelete(keepOnlyFilesPattern: string) {
  return invoke<DeleteFilesPreview>("preview_keep_only_files_delete", {
    keepOnlyFilesPattern,
  });
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

export function disableSongIniFile(relativePath: string) {
  return invoke<SongIniDisableResult>("disable_song_ini_file", {
    relativePath,
  });
}

export function deleteSongIniConflictFile(relativePath: string) {
  return invoke<SongIniDeleteResult>("delete_song_ini_conflict_file", {
    relativePath,
  });
}
