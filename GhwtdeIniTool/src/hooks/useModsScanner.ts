import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import {
  analyzeScannedSongInstruments,
  applyGameIconSongFixes,
  categorizeSongs,
  deleteSongIniConflictFile,
  disableSongIniFile,
  enableSongIniFile,
  fixGameIconCategories,
  previewGameIconSongFixes,
  restoreOriginalSongIni,
  scanGameIconCategories,
  scanSongIniFiles,
  setScannedSongIncluded,
  undoSongIniRepair,
  updateScannedSongMetadata,
  validateSongIniFile,
  verifyContentIssueSongs as verifyContentIssueSongsApi,
} from "../services/scanModsApi";
import type {
  FaultySongIniFile,
  CategorizeSongsInput,
  CategorizeSongsResult,
  GameIconCategoryScanResult,
  GameIconSongFixApplyResult,
  GameIconSongFixInput,
  GameIconSongFixPreview,
  InstrumentAnalyzeMode,
  InstrumentAnalyzeProgress,
  InstrumentAnalyzeResult,
  ScannedSongMetadata,
  SongScanProgress,
  SongIniScanResult,
  SongIniValidationResult,
} from "../types/scanMods";

export type ScanModsStatus =
  | "idle"
  | "scanningSongs"
  | "readySongRepair"
  | "validatingSong"
  | "undoingSong"
  | "verifyingSong"
  | "disablingSong"
  | "enablingSong"
  | "excludingSong"
  | "deletingSongConflict"
  | "error";

export type ScanToast = {
  message: string;
  tone: "success" | "error";
};

export type InstrumentAnalyzeStatus =
  | "idle"
  | "selecting"
  | "analyzing"
  | "complete"
  | "error";

export type GameIconCategoryStatus =
  | "idle"
  | "scanning"
  | "ready"
  | "fixing"
  | "previewing"
  | "applying"
  | "error";

export type CategorizeSongsStatus = "idle" | "categorizing" | "error";

function waitForNextFrame() {
  return new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
}

function addPath(paths: string[], nextPath: string) {
  return paths.includes(nextPath) ? paths : [...paths, nextPath];
}

function removePath(paths: string[], pathToRemove: string) {
  return paths.filter((path) => path !== pathToRemove);
}

function hasIncludedDuplicateChecksumConflict(
  result: SongIniScanResult,
  includedPathSet: Set<string>,
  disabledPathSet: Set<string>,
  deletedPathSet: Set<string>,
) {
  return result.duplicate_checksum_groups.some(
    (group) =>
      group.relative_paths.filter(
        (path) =>
          includedPathSet.has(path) &&
          !disabledPathSet.has(path) &&
          !deletedPathSet.has(path),
      ).length > 1,
  );
}

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [songIniScanResult, setSongIniScanResult] =
    useState<SongIniScanResult | null>(null);
  const [originalFaultySongIniFiles, setOriginalFaultySongIniFiles] = useState<
    Record<string, FaultySongIniFile>
  >({});
  const [repairedSongIniPaths, setRepairedSongIniPaths] = useState<string[]>(
    [],
  );
  const [disabledSongIniPaths, setDisabledSongIniPaths] = useState<string[]>(
    [],
  );
  const [deletedSongIniConflictPaths, setDeletedSongIniConflictPaths] =
    useState<string[]>([]);
  const [includedSongPaths, setIncludedSongPaths] = useState<string[]>([]);
  const [savingScannedSongPaths, setSavingScannedSongPaths] = useState<
    string[]
  >([]);
  const [restoringScannedSongPaths, setRestoringScannedSongPaths] = useState<
    string[]
  >([]);
  const [songIniValidationError, setSongIniValidationError] = useState("");
  const [songIniConflictError, setSongIniConflictError] = useState("");
  const [scanError, setScanError] = useState("");
  const [songScanProgress, setSongScanProgress] =
    useState<SongScanProgress | null>(null);
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
  const [isScanWizardConflictsOnly, setIsScanWizardConflictsOnly] =
    useState(false);
  const [hasCompletedSongScan, setHasCompletedSongScan] = useState(false);
  const [scanToast, setScanToast] = useState<ScanToast | null>(null);
  const [isInstrumentWizardOpen, setIsInstrumentWizardOpen] = useState(false);
  const [instrumentAnalyzeStatus, setInstrumentAnalyzeStatus] =
    useState<InstrumentAnalyzeStatus>("idle");
  const [instrumentAnalyzeProgress, setInstrumentAnalyzeProgress] =
    useState<InstrumentAnalyzeProgress | null>(null);
  const [instrumentAnalyzeResult, setInstrumentAnalyzeResult] =
    useState<InstrumentAnalyzeResult | null>(null);
  const [instrumentAnalyzeError, setInstrumentAnalyzeError] = useState("");
  const [isGameIconWizardOpen, setIsGameIconWizardOpen] = useState(false);
  const [gameIconCategoryStatus, setGameIconCategoryStatus] =
    useState<GameIconCategoryStatus>("idle");
  const [gameIconCategoryResult, setGameIconCategoryResult] =
    useState<GameIconCategoryScanResult | null>(null);
  const [gameIconSongFixPreview, setGameIconSongFixPreview] =
    useState<GameIconSongFixPreview | null>(null);
  const [gameIconCategoryError, setGameIconCategoryError] = useState("");
  const [categorizeSongsStatus, setCategorizeSongsStatus] =
    useState<CategorizeSongsStatus>("idle");
  const [categorizeSongsError, setCategorizeSongsError] = useState("");
  const knownIncludedSongPathSetRef = useRef(new Set<string>());

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setHasCompletedSongScan(false);
    setScanStatus("scanningSongs");
    setScanError("");
    setSongScanProgress(null);
    setSongIniScanResult(null);
    setOriginalFaultySongIniFiles({});
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setIncludedSongPaths([]);
    setSavingScannedSongPaths([]);
    setRestoringScannedSongPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setScanToast(null);
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");
    setIsGameIconWizardOpen(false);
    setGameIconCategoryStatus("idle");
    setGameIconCategoryResult(null);
    setGameIconSongFixPreview(null);
    setGameIconCategoryError("");
    setIsScanWizardConflictsOnly(false);

    try {
      const nextSongIniScanResult = await scanSongIniFiles();

      setIncludedSongPaths(
        nextSongIniScanResult.songs
          .filter((song) => song.is_included)
          .map((song) => song.relative_path),
      );
      setDisabledSongIniPaths(nextSongIniScanResult.disabled_song_ini_paths);
      setOriginalFaultySongIniFiles(
        Object.fromEntries(
          nextSongIniScanResult.faulty_files.map((file) => [
            file.relative_path,
            file,
          ]),
        ),
      );
      setSongIniScanResult(nextSongIniScanResult);
      setSongScanProgress(null);
      setScanStatus("readySongRepair");
    } catch (err) {
      setSongIniScanResult(null);
      setOriginalFaultySongIniFiles({});
      setRepairedSongIniPaths([]);
      setDisabledSongIniPaths([]);
      setDeletedSongIniConflictPaths([]);
      setIncludedSongPaths([]);
      setSavingScannedSongPaths([]);
      setRestoringScannedSongPaths([]);
      setSongIniValidationError("");
      setSongIniConflictError("");
      setScanError(String(err));
      setSongScanProgress(null);
      setScanStatus("error");
    }
  }, []);

  const confirmScan = useCallback(async () => {
    if (scanStatus === "readySongRepair") {
      setIsScanWizardOpen(false);
      setIsScanWizardConflictsOnly(false);
      setHasCompletedSongScan(true);
      setOriginalFaultySongIniFiles({});
      setScanStatus("idle");
    }
  }, [scanStatus]);

  const applySongIniValidationResult = useCallback(
    (result: SongIniValidationResult) => {
      setSongIniScanResult((currentResult) =>
        currentResult === null
          ? currentResult
          : {
              ...currentResult,
              songs_parsed: result.songs_parsed,
              songs: result.songs,
              duplicate_checksum_groups: result.duplicate_checksum_groups,
              song_ini_folder_conflicts: result.song_ini_folder_conflicts,
              content_file_issues: result.content_file_issues,
            },
      );
    },
    [],
  );

  const applyGameIconSongFixApplyResult = useCallback(
    (result: GameIconSongFixApplyResult) => {
      setSongIniScanResult((currentResult) =>
        currentResult === null
          ? currentResult
          : {
              ...currentResult,
              songs_parsed: result.songs_parsed,
              songs: result.songs,
              duplicate_checksum_groups: result.duplicate_checksum_groups,
              song_ini_folder_conflicts: result.song_ini_folder_conflicts,
              content_file_issues: result.content_file_issues,
            },
      );
    },
    [],
  );

  const applyCategorizeSongsResult = useCallback((result: CategorizeSongsResult) => {
    setSongIniScanResult((currentResult) =>
      currentResult === null
        ? currentResult
        : {
            ...currentResult,
            songs_parsed: result.songs_parsed,
            songs: result.songs,
            duplicate_checksum_groups: result.duplicate_checksum_groups,
            song_ini_folder_conflicts: result.song_ini_folder_conflicts,
            content_file_issues: result.content_file_issues,
          },
    );
    setIncludedSongPaths(
      result.songs
        .filter((song) => song.is_included)
        .map((song) => song.relative_path),
    );
  }, []);

  const validateSongIni = useCallback(
    async (relativePath: string, contents: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("validatingSong");
      setSongIniValidationError("");
      setSongIniConflictError("");

      try {
        const result = await validateSongIniFile(relativePath, contents);
        setRepairedSongIniPaths((currentPaths) =>
          addPath(currentPaths, relativePath),
        );
        setSongIniScanResult((currentResult) =>
          currentResult === null
            ? currentResult
            : {
              ...currentResult,
              songs_parsed: result.songs_parsed,
              songs: result.songs,
              duplicate_checksum_groups: result.duplicate_checksum_groups,
              song_ini_folder_conflicts: result.song_ini_folder_conflicts,
              content_file_issues: result.content_file_issues,
              faulty_files: currentResult.faulty_files.map((file) =>
                file.relative_path === relativePath
                  ? { ...file, contents: result.contents, error: "" }
                  : file,
              ),
            },
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${relativePath} validated and saved`,
          tone: "success",
        });
      } catch (err) {
        const error = String(err);

        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
                ...currentResult,
                faulty_files: currentResult.faulty_files.map((file) =>
                  file.relative_path === relativePath
                    ? { ...file, contents, error }
                    : file,
                ),
              }
            : currentResult,
        );
        setSongIniValidationError("");
        setScanStatus("readySongRepair");
      }
    },
    [scanStatus],
  );

  const verifyContentIssueSongs = useCallback(
    async (relativePaths: string[]) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("verifyingSong");
      setSongIniConflictError("");

      try {
        const result = await verifyContentIssueSongsApi(relativePaths);

        setSongIniScanResult((currentResult) =>
          currentResult === null
            ? currentResult
            : {
                ...currentResult,
                songs_parsed: result.songs_parsed,
                songs: result.songs,
                duplicate_checksum_groups: result.duplicate_checksum_groups,
                song_ini_folder_conflicts: result.song_ini_folder_conflicts,
                content_file_issues: result.content_file_issues,
              },
        );
        setIncludedSongPaths(
          result.songs
            .filter((song) => song.is_included)
            .map((song) => song.relative_path),
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: "Content issues verified",
          tone: "success",
        });
      } catch (err) {
        setSongIniConflictError(String(err));
        setScanStatus("readySongRepair");
      }
    },
    [scanStatus],
  );

  const undoSongIni = useCallback(
    async (relativePath: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      const originalFile = originalFaultySongIniFiles[relativePath];

      if (!originalFile) {
        setScanToast({
          message: `${relativePath} has no undo snapshot`,
          tone: "error",
        });
        return;
      }

      setScanStatus("undoingSong");
      setSongIniValidationError("");
      setSongIniConflictError("");

      try {
        const result = await undoSongIniRepair(
          relativePath,
          originalFile.contents,
        );
        setRepairedSongIniPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
        setSongIniScanResult((currentResult) =>
          currentResult === null
            ? currentResult
            : {
                ...currentResult,
                songs_parsed: result.songs_parsed,
                songs: result.songs,
                duplicate_checksum_groups: result.duplicate_checksum_groups,
                song_ini_folder_conflicts: result.song_ini_folder_conflicts,
                content_file_issues: result.content_file_issues,
                faulty_files: currentResult.faulty_files.map((file) =>
                  file.relative_path === relativePath ? originalFile : file,
                ),
              },
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${relativePath} restored to the scan-time contents`,
          tone: "success",
        });
      } catch (err) {
        setSongIniConflictError(String(err));
        setScanStatus("readySongRepair");
      }
    },
    [originalFaultySongIniFiles, scanStatus],
  );

  const saveScannedSongMetadata = useCallback(
    async (
      relativePath: string,
      metadata: ScannedSongMetadata,
      isIncluded: boolean,
    ) => {
      setSavingScannedSongPaths((currentPaths) =>
        addPath(currentPaths, relativePath),
      );

      try {
        const result = await updateScannedSongMetadata(
          relativePath,
          metadata,
          isIncluded,
        );

        applySongIniValidationResult(result);
        setIncludedSongPaths((currentPaths) => {
          const nextPaths = removePath(currentPaths, relativePath);

          return isIncluded
            ? addPath(nextPaths, result.relative_path)
            : nextPaths;
        });
        setScanToast({
          message: `${result.relative_path} metadata saved`,
          tone: "success",
        });
      } catch (err) {
        setScanToast({
          message: `Failed to save ${relativePath}: ${String(err)}`,
          tone: "error",
        });
        throw err;
      } finally {
        setSavingScannedSongPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
      }
    },
    [applySongIniValidationResult],
  );

  const restoreScannedSongOriginal = useCallback(
    async (relativePath: string, isIncluded: boolean) => {
      setRestoringScannedSongPaths((currentPaths) =>
        addPath(currentPaths, relativePath),
      );

      try {
        const result = await restoreOriginalSongIni(relativePath, isIncluded);

        applySongIniValidationResult(result);
        setIncludedSongPaths((currentPaths) => {
          const nextPaths = removePath(currentPaths, relativePath);

          return isIncluded
            ? addPath(nextPaths, result.relative_path)
            : nextPaths;
        });
        setScanToast({
          message: `${relativePath} restored from original`,
          tone: "success",
        });
      } catch (err) {
        setScanToast({
          message: `Failed to restore ${relativePath}: ${String(err)}`,
          tone: "error",
        });
        throw err;
      } finally {
        setRestoringScannedSongPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
      }
    },
    [applySongIniValidationResult],
  );

  const clearSongIniValidationError = useCallback(() => {
    setSongIniValidationError("");
    setSongIniConflictError("");
  }, []);

  const clearCompletedSongScan = useCallback(() => {
    setHasCompletedSongScan(false);
    setSongIniScanResult(null);
    setOriginalFaultySongIniFiles({});
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setIncludedSongPaths([]);
    setSavingScannedSongPaths([]);
    setRestoringScannedSongPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setScanError("");
    setSongScanProgress(null);
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");
    setIsGameIconWizardOpen(false);
    setGameIconCategoryStatus("idle");
    setGameIconCategoryResult(null);
    setGameIconSongFixPreview(null);
    setGameIconCategoryError("");
    setIsScanWizardConflictsOnly(false);
  }, []);

  const copyContentIssuePath = useCallback(async (absolutePath: string) => {
    try {
      await navigator.clipboard.writeText(absolutePath);
      setScanToast({
        message: `${absolutePath} was copied to clipboard`,
        tone: "success",
      });
    } catch (err) {
      setScanToast({
        message: `Failed to copy path: ${String(err)}`,
        tone: "error",
      });
    }
  }, []);

  const setSongIncluded = useCallback(
    (relativePath: string, isIncluded: boolean) => {
      setIncludedSongPaths((currentPaths) => {
        const nextPathSet = new Set(currentPaths);

        if (isIncluded) {
          nextPathSet.add(relativePath);
        } else {
          nextPathSet.delete(relativePath);
        }

        return Array.from(nextPathSet);
      });
    },
    [],
  );

  const setSongsIncluded = useCallback(
    (relativePaths: string[], isIncluded: boolean) => {
      setIncludedSongPaths((currentPaths) => {
        const nextPathSet = new Set(currentPaths);

        for (const relativePath of relativePaths) {
          if (isIncluded) {
            nextPathSet.add(relativePath);
          } else {
            nextPathSet.delete(relativePath);
          }
        }

        return Array.from(nextPathSet);
      });
    },
    [],
  );

  const setDuplicateSongIncluded = useCallback(
    async (relativePath: string, isIncluded: boolean) => {
      if (scanStatus !== "readySongRepair") {
        return null;
      }

      setScanStatus("excludingSong");
      setSongIniConflictError("");

      try {
        const result = await setScannedSongIncluded(relativePath, isIncluded);

        applySongIniValidationResult(result);
        setDeletedSongIniConflictPaths((currentPaths) =>
          removePath(currentPaths, result.relative_path),
        );
        setIncludedSongPaths((currentPaths) => {
          const nextPaths = removePath(currentPaths, relativePath);

          return isIncluded
            ? addPath(nextPaths, result.relative_path)
            : nextPaths;
        });
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${result.relative_path} ${isIncluded ? "included" : "excluded"}`,
          tone: "success",
        });
        return result.relative_path;
      } catch (err) {
        setSongIniConflictError(String(err));
        setScanStatus("readySongRepair");
        return null;
      }
    },
    [applySongIniValidationResult, scanStatus],
  );

  const disableSongIni = useCallback(
    async (relativePath: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("disablingSong");
      setSongIniValidationError("");
      setSongIniConflictError("");

      try {
        const result = await disableSongIniFile(relativePath);
        setDisabledSongIniPaths((currentPaths) =>
          addPath(currentPaths, relativePath),
        );
        setIncludedSongPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
                ...currentResult,
                songs_parsed: result.songs_parsed,
                songs: currentResult.songs.filter(
                  (song) => song.relative_path !== relativePath,
                ),
              }
            : currentResult,
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${relativePath} disabled as ${result.disabled_path}`,
          tone: "success",
        });
      } catch (err) {
        const error = String(err);

        setSongIniConflictError(error);
        setScanStatus("readySongRepair");
      }
    },
    [scanStatus],
  );

  const enableSongIni = useCallback(
    async (relativePath: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("enablingSong");
      setSongIniValidationError("");
      setSongIniConflictError("");

      try {
        const result = await enableSongIniFile(relativePath);
        const refreshedResult = await verifyContentIssueSongsApi([relativePath]);
        setDisabledSongIniPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
        setIncludedSongPaths(
          refreshedResult.songs
            .filter((song) => song.is_included)
            .map((song) => song.relative_path),
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
              ...currentResult,
              songs_parsed: refreshedResult.songs_parsed,
              songs: refreshedResult.songs,
              duplicate_checksum_groups: refreshedResult.duplicate_checksum_groups,
              song_ini_folder_conflicts: refreshedResult.song_ini_folder_conflicts,
              content_file_issues: refreshedResult.content_file_issues,
              disabled_song_ini_paths: removePath(
                currentResult.disabled_song_ini_paths,
                relativePath,
              ),
              disabled_content_file_issues:
                currentResult.disabled_content_file_issues.filter(
                  (issue) => issue.song_ini_relative_path !== relativePath,
                ),
              }
            : currentResult,
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${relativePath} enabled as ${result.enabled_path}`,
          tone: "success",
        });
      } catch (err) {
        const error = String(err);

        setSongIniConflictError(error);
        setScanStatus("readySongRepair");
      }
    },
    [scanStatus],
  );

  const deleteSongIniConflict = useCallback(
    async (relativePath: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("deletingSongConflict");
      setSongIniConflictError("");

      try {
        const result = await deleteSongIniConflictFile(relativePath);
        setDeletedSongIniConflictPaths((currentPaths) =>
          addPath(currentPaths, relativePath),
        );
        setIncludedSongPaths((currentPaths) =>
          removePath(currentPaths, relativePath),
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
                ...currentResult,
                songs_parsed: result.songs_parsed,
                songs: currentResult.songs.filter(
                  (song) => song.relative_path !== relativePath,
                ),
              }
            : currentResult,
        );
        setScanStatus("readySongRepair");
        setScanToast({
          message: `${relativePath} deleted`,
          tone: "success",
        });
      } catch (err) {
        const error = String(err);

        setSongIniConflictError(error);
        setScanStatus("readySongRepair");
      }
    },
    [scanStatus],
  );

  const cancelScan = useCallback(() => {
    if (isScanWizardConflictsOnly) {
      setIsScanWizardOpen(false);
      setIsScanWizardConflictsOnly(false);
      setScanStatus("idle");
      setSongIniValidationError("");
      setSongIniConflictError("");
      return;
    }

    setIsScanWizardOpen(false);
    setHasCompletedSongScan(false);
    setScanStatus("idle");
    setScanError("");
    setSongScanProgress(null);
    setSongIniScanResult(null);
    setOriginalFaultySongIniFiles({});
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setIncludedSongPaths([]);
    setSavingScannedSongPaths([]);
    setRestoringScannedSongPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setIsGameIconWizardOpen(false);
    setGameIconCategoryStatus("idle");
    setGameIconCategoryResult(null);
    setGameIconSongFixPreview(null);
    setGameIconCategoryError("");
    setIsScanWizardConflictsOnly(false);
  }, [isScanWizardConflictsOnly]);

  const openInstrumentAnalyzer = useCallback(() => {
    setIsInstrumentWizardOpen(true);
    setIsGameIconWizardOpen(false);
    setInstrumentAnalyzeStatus("selecting");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");
  }, []);

  const cancelInstrumentAnalyzer = useCallback(() => {
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeError("");
  }, []);

  const closeInstrumentAnalyzer = useCallback(() => {
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");
  }, []);

  const openGameIconCategories = useCallback(async () => {
    setIsGameIconWizardOpen(true);
    setIsInstrumentWizardOpen(false);
    setGameIconCategoryStatus("scanning");
    setGameIconCategoryResult(null);
    setGameIconSongFixPreview(null);
    setGameIconCategoryError("");

    try {
      await waitForNextFrame();
      const result = await scanGameIconCategories();

      setGameIconCategoryResult(result);
      setGameIconCategoryStatus("ready");
    } catch (err) {
      setGameIconCategoryError(String(err));
      setGameIconCategoryStatus("error");
    }
  }, []);

  const closeGameIconCategories = useCallback(() => {
    setIsGameIconWizardOpen(false);
    setGameIconCategoryStatus("idle");
    setGameIconCategoryResult(null);
    setGameIconSongFixPreview(null);
    setGameIconCategoryError("");
  }, []);

  const fixGameIconCategoryFolders = useCallback(async () => {
    setGameIconCategoryStatus("fixing");
    setGameIconCategoryError("");
    setGameIconSongFixPreview(null);

    try {
      await waitForNextFrame();
      const result = await fixGameIconCategories();

      setGameIconCategoryResult(result);
      setGameIconCategoryStatus("ready");
    } catch (err) {
      setGameIconCategoryError(String(err));
      setGameIconCategoryStatus("error");
    }
  }, []);

  const previewGameIconSongFixRows = useCallback(async () => {
    setGameIconCategoryStatus("previewing");
    setGameIconCategoryError("");
    setGameIconSongFixPreview(null);

    try {
      await waitForNextFrame();
      const result = await previewGameIconSongFixes();

      setGameIconSongFixPreview(result);
      setGameIconCategoryStatus("ready");
    } catch (err) {
      setGameIconCategoryError(String(err));
      setGameIconCategoryStatus("error");
    }
  }, []);

  const applyGameIconSongFixRows = useCallback(
    async (fixes: GameIconSongFixInput[]) => {
      setGameIconCategoryStatus("applying");
      setGameIconCategoryError("");

      try {
        await waitForNextFrame();
        const result = await applyGameIconSongFixes(fixes);

        applyGameIconSongFixApplyResult(result);
        setIsGameIconWizardOpen(false);
        setGameIconCategoryStatus("idle");
        setGameIconCategoryResult(null);
        setGameIconSongFixPreview(null);
        setScanToast({
          message: `${result.applied} GameIcon fixes applied`,
          tone: "success",
        });
      } catch (err) {
        setGameIconCategoryError(String(err));
        setGameIconCategoryStatus("error");
        throw err;
      }
    },
    [applyGameIconSongFixApplyResult],
  );

  const applyCategorization = useCallback(
    async (input: CategorizeSongsInput) => {
      setCategorizeSongsStatus("categorizing");
      setCategorizeSongsError("");

      try {
        await waitForNextFrame();
        const result = await categorizeSongs(input);

        applyCategorizeSongsResult(result);
        setCategorizeSongsStatus("idle");
        setScanToast({
          message: `${result.categorized} songs categorized; ${result.excluded} excluded`,
          tone: "success",
        });
      } catch (err) {
        setCategorizeSongsError(String(err));
        setCategorizeSongsStatus("error");
        throw err;
      }
    },
    [applyCategorizeSongsResult],
  );

  const analyzeInstruments = useCallback(async (mode: InstrumentAnalyzeMode) => {
    setInstrumentAnalyzeStatus("analyzing");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");

    try {
      await waitForNextFrame();
      const result = await analyzeScannedSongInstruments(mode);

      setInstrumentAnalyzeResult(result);
      setSongIniScanResult((currentResult) =>
        currentResult
          ? {
              ...currentResult,
              songs: result.songs,
            }
          : currentResult,
      );
      setInstrumentAnalyzeStatus("complete");
    } catch (err) {
      setInstrumentAnalyzeError(String(err));
      setInstrumentAnalyzeStatus("error");
    }
  }, []);

  const dismissScanToast = useCallback(() => {
    setScanToast(null);
  }, []);

  useEffect(() => {
    if (!songIniScanResult) {
      knownIncludedSongPathSetRef.current = new Set();
      setIncludedSongPaths([]);
      return;
    }

    const currentSongPathSet = new Set(
      songIniScanResult.songs.map((song) => song.relative_path),
    );
    const previousSongPathSet = knownIncludedSongPathSetRef.current;

    setIncludedSongPaths((currentPaths) => {
      const currentPathSet = new Set(currentPaths);
      const nextPathSet = new Set<string>();

      for (const song of songIniScanResult.songs) {
        if (
          currentPathSet.has(song.relative_path) ||
          (!previousSongPathSet.has(song.relative_path) && song.is_included)
        ) {
          nextPathSet.add(song.relative_path);
        }
      }

      return Array.from(nextPathSet);
    });

    knownIncludedSongPathSetRef.current = currentSongPathSet;
  }, [songIniScanResult]);

  useEffect(() => {
    if (
      !hasCompletedSongScan ||
      !songIniScanResult ||
      isScanWizardOpen ||
      !hasIncludedDuplicateChecksumConflict(
        songIniScanResult,
        new Set(includedSongPaths),
        new Set(disabledSongIniPaths),
        new Set(deletedSongIniConflictPaths),
      )
    ) {
      return;
    }

    setIsScanWizardOpen(true);
    setIsScanWizardConflictsOnly(true);
    setIsInstrumentWizardOpen(false);
    setIsGameIconWizardOpen(false);
    setScanStatus("readySongRepair");
    setSongIniConflictError("");
  }, [
    deletedSongIniConflictPaths,
    disabledSongIniPaths,
    hasCompletedSongScan,
    includedSongPaths,
    isScanWizardOpen,
    songIniScanResult,
  ]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let isMounted = true;

    listen<InstrumentAnalyzeProgress>(
      "instrument_scan_progress",
      (event) => {
        if (isMounted) {
          setInstrumentAnalyzeProgress(event.payload);
        }
      },
    ).then((nextUnlisten) => {
      if (isMounted) {
        unlisten = nextUnlisten;
      } else {
        nextUnlisten();
      }
    });

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let isMounted = true;

    listen<SongScanProgress>("song_scan_progress", (event) => {
      if (isMounted) {
        setSongScanProgress(event.payload);
      }
    }).then((nextUnlisten) => {
      if (isMounted) {
        unlisten = nextUnlisten;
      } else {
        nextUnlisten();
      }
    });

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!scanToast) {
      return;
    }

    const timeout = window.setTimeout(() => {
      setScanToast(null);
    }, 5000);

    return () => window.clearTimeout(timeout);
  }, [scanToast]);

  return {
    analyzeInstruments,
    applyCategorization,
    applyGameIconSongFixRows,
    cancelScan,
    categorizeSongsError,
    categorizeSongsStatus,
    cancelInstrumentAnalyzer,
    clearCompletedSongScan,
    clearSongIniValidationError,
    closeInstrumentAnalyzer,
    confirmScan,
    copyContentIssuePath,
    deleteSongIniConflict,
    deletedSongIniConflictPaths,
    disableSongIni,
    disabledSongIniPaths,
    enableSongIni,
    fixGameIconCategoryFolders,
    dismissScanToast,
    gameIconCategoryError,
    gameIconCategoryResult,
    gameIconCategoryStatus,
    gameIconSongFixPreview,
    hasCompletedSongScan,
    includedSongPaths,
    instrumentAnalyzeError,
    instrumentAnalyzeProgress,
    instrumentAnalyzeResult,
    instrumentAnalyzeStatus,
    isGameIconWizardOpen,
    isInstrumentWizardOpen,
    isScanWizardConflictsOnly,
    isScanWizardOpen,
    closeGameIconCategories,
    openInstrumentAnalyzer,
    openGameIconCategories,
    previewGameIconSongFixRows,
    originalFaultySongIniFiles,
    repairedSongIniPaths,
    restoringScannedSongPaths,
    restoreScannedSongOriginal,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    saveScannedSongMetadata,
    savingScannedSongPaths,
    songIniConflictError,
    songIniScanResult,
    songScanProgress,
    songIniValidationError,
    setSongIncluded,
    setDuplicateSongIncluded,
    setSongsIncluded,
    undoSongIni,
    validateSongIni,
    verifyContentIssueSongs,
  };
}
