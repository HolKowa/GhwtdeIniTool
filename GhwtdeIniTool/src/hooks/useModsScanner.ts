import { useCallback, useEffect, useState } from "react";

import {
  deleteKeepOnlyFiles,
  previewKeepOnlyFilesDelete,
  scanSongIniFiles,
  validateSongIniFile,
} from "../services/scanModsApi";
import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  SongIniScanResult,
} from "../types/scanMods";

export type ScanModsStatus =
  | "idle"
  | "previewing"
  | "readyDelete"
  | "deleting"
  | "scanningSongs"
  | "readySongRepair"
  | "validatingSong"
  | "error";

export type ScanToast = {
  message: string;
  tone: "success" | "error";
};

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [deletePreview, setDeletePreview] =
    useState<DeleteFilesPreview | null>(null);
  const [deleteResult, setDeleteResult] = useState<DeleteFilesResult | null>(
    null,
  );
  const [songIniScanResult, setSongIniScanResult] =
    useState<SongIniScanResult | null>(null);
  const [repairedSongIniPaths, setRepairedSongIniPaths] = useState<string[]>(
    [],
  );
  const [songIniValidationError, setSongIniValidationError] = useState("");
  const [scanError, setScanError] = useState("");
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
  const [scanToast, setScanToast] = useState<ScanToast | null>(null);

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setScanStatus("previewing");
    setScanError("");
    setDeletePreview(null);
    setDeleteResult(null);
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setSongIniValidationError("");
    setScanToast(null);

    try {
      const preview = await previewKeepOnlyFilesDelete();
      setDeletePreview(preview);
      setScanStatus("readyDelete");
    } catch (err) {
      setDeletePreview(null);
      setDeleteResult(null);
      setSongIniScanResult(null);
      setRepairedSongIniPaths([]);
      setSongIniValidationError("");
      setScanError(String(err));
      setScanStatus("error");
    }
  }, []);

  const confirmScan = useCallback(async () => {
    if (scanStatus === "readySongRepair") {
      setIsScanWizardOpen(false);
      setScanStatus("idle");
      return;
    }

    if (scanStatus === "readyDelete") {
      setScanStatus("deleting");
      setScanError("");
      setSongIniValidationError("");

      try {
        const result = await deleteKeepOnlyFiles(
          deletePreview?.files_to_delete ?? [],
        );
        setDeleteResult(result);
        setScanStatus("scanningSongs");
        const nextSongIniScanResult = await scanSongIniFiles();
        setSongIniScanResult(nextSongIniScanResult);
        setRepairedSongIniPaths([]);
        setScanStatus("readySongRepair");
        setScanToast({
          message: [
            `${result.files_deleted} files deleted`,
            `${nextSongIniScanResult.songs_parsed} songs parsed`,
            `${nextSongIniScanResult.faulty_files.length} song.ini errors`,
          ].join(" | "),
          tone:
            result.errors.length ||
            nextSongIniScanResult.errors.length ||
            nextSongIniScanResult.faulty_files.length
              ? "error"
              : "success",
        });
      } catch (err) {
        setDeleteResult(null);
        setSongIniScanResult(null);
        setRepairedSongIniPaths([]);
        setScanError(String(err));
        setScanStatus("error");
        setScanToast({
          message: String(err),
          tone: "error",
        });
      }

      return;
    }
  }, [deletePreview, scanStatus]);

  const validateSongIni = useCallback(
    async (relativePath: string, contents: string) => {
      if (scanStatus !== "readySongRepair") {
        return;
      }

      setScanStatus("validatingSong");
      setSongIniValidationError("");

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
  }, []);

  const cancelScan = useCallback(() => {
    setIsScanWizardOpen(false);
    setScanStatus("idle");
    setScanError("");
    setDeletePreview(null);
    setDeleteResult(null);
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setSongIniValidationError("");
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
    deletePreview,
    deleteResult,
    dismissScanToast,
    isScanWizardOpen,
    repairedSongIniPaths,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    songIniScanResult,
    songIniValidationError,
    validateSongIni,
  };
}
