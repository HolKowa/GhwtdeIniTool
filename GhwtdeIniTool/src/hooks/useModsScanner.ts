import { useCallback, useEffect, useState } from "react";

import {
  scanSongIniFiles,
  validateSongIniFile,
} from "../services/scanModsApi";
import type { SongIniScanResult } from "../types/scanMods";

export type ScanModsStatus =
  | "idle"
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
    setScanStatus("scanningSongs");
    setScanError("");
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setSongIniValidationError("");
    setScanToast(null);

    try {
      const nextSongIniScanResult = await scanSongIniFiles();
      setSongIniScanResult(nextSongIniScanResult);
      setScanStatus("readySongRepair");
      setScanToast({
        message: [
          `${nextSongIniScanResult.songs_parsed} songs parsed`,
          `${nextSongIniScanResult.faulty_files.length} song.ini errors`,
        ].join(" | "),
        tone:
          nextSongIniScanResult.errors.length ||
          nextSongIniScanResult.faulty_files.length
            ? "error"
            : "success",
      });
    } catch (err) {
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
    }
  }, [scanStatus]);

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
