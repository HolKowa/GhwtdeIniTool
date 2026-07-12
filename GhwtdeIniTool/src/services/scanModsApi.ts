import { invoke } from "@tauri-apps/api/core";

import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  GameIconCategoryScanResult,
  GameIconSongFixApplyResult,
  GameIconSongFixInput,
  GameIconSongFixPreview,
  InstrumentAnalyzeMode,
  InstrumentAnalyzeResult,
  IniToolCategoriesPreview,
  RestoreIniAction,
  RestoreIniResult,
  ScannedSongMetadata,
  SongIniDeleteResult,
  SongIniDisableResult,
  SongIniEnableResult,
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

export function previewIniToolCategories() {
  return invoke<IniToolCategoriesPreview>("preview_ini_tool_categories");
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

export function undoSongIniRepair(relativePath: string, contents: string) {
  return invoke<SongIniValidationResult>("undo_song_ini_repair", {
    relativePath,
    contents,
  });
}

export function verifySongIniFile(relativePath: string) {
  return invoke<SongIniValidationResult>("verify_song_ini_file", {
    relativePath,
  });
}

export function disableSongIniFile(relativePath: string) {
  return invoke<SongIniDisableResult>("disable_song_ini_file", {
    relativePath,
  });
}

export function enableSongIniFile(relativePath: string) {
  return invoke<SongIniEnableResult>("enable_song_ini_file", {
    relativePath,
  });
}

export function deleteSongIniConflictFile(relativePath: string) {
  return invoke<SongIniDeleteResult>("delete_song_ini_conflict_file", {
    relativePath,
  });
}

export function setScannedSongIncluded(relativePath: string, isIncluded: boolean) {
  return invoke<SongIniValidationResult>("set_scanned_song_included", {
    relativePath,
    isIncluded,
  });
}

export function updateScannedSongMetadata(
  relativePath: string,
  metadata: ScannedSongMetadata,
  isIncluded: boolean,
) {
  return invoke<SongIniValidationResult>("update_scanned_song_metadata", {
    relativePath,
    metadata,
    isIncluded,
  });
}

export function restoreOriginalSongIni(relativePath: string, isIncluded: boolean) {
  return invoke<SongIniValidationResult>("restore_original_song_ini", {
    relativePath,
    isIncluded,
  });
}

export function restoreAllOriginalSongIni(action: RestoreIniAction) {
  return invoke<RestoreIniResult>("restore_all_original_song_ini", {
    action,
  });
}

export function analyzeScannedSongInstruments(mode: InstrumentAnalyzeMode) {
  return invoke<InstrumentAnalyzeResult>("analyze_scanned_song_instruments", {
    mode,
  });
}

export function scanGameIconCategories() {
  return invoke<GameIconCategoryScanResult>("scan_game_icon_categories");
}

export function fixGameIconCategories() {
  return invoke<GameIconCategoryScanResult>("fix_game_icon_categories");
}

export function previewGameIconSongFixes() {
  return invoke<GameIconSongFixPreview>("preview_game_icon_song_fixes");
}

export function applyGameIconSongFixes(fixes: GameIconSongFixInput[]) {
  return invoke<GameIconSongFixApplyResult>("apply_game_icon_song_fixes", {
    fixes,
  });
}
