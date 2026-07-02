import { useCallback, useEffect, useState } from "react";

import {
  deleteSongIniConflictFile,
  disableSongIniFile,
  scanSongIniFiles,
  validateSongIniFile,
} from "../services/scanModsApi";
import type { SongIniScanResult } from "../types/scanMods";

export type ScanModsStatus =
  | "idle"
  | "scanningSongs"
  | "readySongRepair"
  | "validatingSong"
  | "disablingSong"
  | "deletingSongConflict"
  | "error";

export type ScanToast = {
  message: string;
  tone: "success" | "error";
};

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [songIniScanResult, setSongIniScanResult] =
    useState<SongIniScanResult | null>(null);
  const [repairedSongIniPaths, setRepairedSongIniPaths] = useState<string[]>(
    [],
  );
  const [disabledSongIniPaths, setDisabledSongIniPaths] = useState<string[]>(
    [],
  );
  const [deletedSongIniConflictPaths, setDeletedSongIniConflictPaths] =
    useState<string[]>([]);
  const [songIniValidationError, setSongIniValidationError] = useState("");
  const [songIniConflictError, setSongIniConflictError] = useState("");
  const [scanError, setScanError] = useState("");
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
  const [scanToast, setScanToast] = useState<ScanToast | null>(null);

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setScanStatus("scanningSongs");
    setScanError("");
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setScanToast(null);

    try {
      const nextSongIniScanResult = await scanSongIniFiles();
      const hasWizardIssues =
        nextSongIniScanResult.faulty_files.length > 0 ||
        nextSongIniScanResult.duplicate_checksum_groups.length > 0 ||
        nextSongIniScanResult.disabled_song_conflicts.length > 0 ||
        nextSongIniScanResult.content_file_issues.length > 0;

      setSongIniScanResult(nextSongIniScanResult);
      setScanStatus("readySongRepair");

      if (!hasWizardIssues) {
        setScanToast({
          message: `${nextSongIniScanResult.songs_parsed} songs parsed with no issues`,
          tone: nextSongIniScanResult.errors.length ? "error" : "success",
        });
      }
    } catch (err) {
      setSongIniScanResult(null);
      setRepairedSongIniPaths([]);
      setDisabledSongIniPaths([]);
      setDeletedSongIniConflictPaths([]);
      setSongIniValidationError("");
      setSongIniConflictError("");
      setScanError(String(err));
      setScanStatus("error");
    }
  }, []);

  const confirmScan = useCallback(async () => {
    if (scanStatus === "readySongRepair") {
      setIsScanWizardOpen(false);
      setScanStatus("idle");
    }
  }, [scanStatus]);

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
          currentPaths.includes(relativePath)
            ? currentPaths
            : [...currentPaths, relativePath],
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
              ...currentResult,
              songs_parsed: result.songs_parsed,
              duplicate_checksum_groups: result.duplicate_checksum_groups,
              disabled_song_conflicts: result.disabled_song_conflicts,
              content_file_issues: result.content_file_issues,
              faulty_files: currentResult.faulty_files.map((file) =>
                file.relative_path === relativePath
                    ? { ...file, contents: result.contents, error: "" }
                    : file,
                ),
              }
            : currentResult,
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

  const clearSongIniValidationError = useCallback(() => {
    setSongIniValidationError("");
    setSongIniConflictError("");
  }, []);

  const copyContentIssuePath = useCallback(async (absolutePath: string) => {
    try {
      await navigator.clipboard.writeText(absolutePath);
      setScanToast({
        message: "Path copied to clipboard",
        tone: "success",
      });
    } catch (err) {
      setScanToast({
        message: `Failed to copy path: ${String(err)}`,
        tone: "error",
      });
    }
  }, []);

  const disableSongIni = useCallback(
    async (relativePath: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("disablingSong");
      setSongIniConflictError("");

      try {
        const result = await disableSongIniFile(relativePath);
        setDisabledSongIniPaths((currentPaths) =>
          currentPaths.includes(relativePath)
            ? currentPaths
            : [...currentPaths, relativePath],
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
                ...currentResult,
                songs_parsed: result.songs_parsed,
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
          currentPaths.includes(relativePath)
            ? currentPaths
            : [...currentPaths, relativePath],
        );
        setSongIniScanResult((currentResult) =>
          currentResult
            ? {
                ...currentResult,
                songs_parsed: result.songs_parsed,
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
    setIsScanWizardOpen(false);
    setScanStatus("idle");
    setScanError("");
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
  }, []);

  const dismissScanToast = useCallback(() => {
    setScanToast(null);
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
    cancelScan,
    clearSongIniValidationError,
    confirmScan,
    copyContentIssuePath,
    deleteSongIniConflict,
    deletedSongIniConflictPaths,
    disableSongIni,
    disabledSongIniPaths,
    dismissScanToast,
    isScanWizardOpen,
    repairedSongIniPaths,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    songIniConflictError,
    songIniScanResult,
    songIniValidationError,
    validateSongIni,
  };
}
