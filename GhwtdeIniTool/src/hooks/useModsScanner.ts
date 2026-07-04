import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import {
  analyzeScannedSongInstruments,
  deleteSongIniConflictFile,
  disableSongIniFile,
  scanSongIniFiles,
  validateSongIniFile,
} from "../services/scanModsApi";
import type {
  InstrumentAnalyzeMode,
  InstrumentAnalyzeProgress,
  InstrumentAnalyzeResult,
  SongScanProgress,
  SongIniScanResult,
} from "../types/scanMods";

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

export type InstrumentAnalyzeStatus =
  | "idle"
  | "selecting"
  | "analyzing"
  | "complete"
  | "error";

function waitForNextFrame() {
  return new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
}

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
  const [songScanProgress, setSongScanProgress] =
    useState<SongScanProgress | null>(null);
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
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

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setHasCompletedSongScan(false);
    setScanStatus("scanningSongs");
    setScanError("");
    setSongScanProgress(null);
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setScanToast(null);
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");

    try {
      const nextSongIniScanResult = await scanSongIniFiles();

      setSongIniScanResult(nextSongIniScanResult);
      setSongScanProgress(null);
      setScanStatus("readySongRepair");
    } catch (err) {
      setSongIniScanResult(null);
      setRepairedSongIniPaths([]);
      setDisabledSongIniPaths([]);
      setDeletedSongIniConflictPaths([]);
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
      setHasCompletedSongScan(true);
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
          currentResult === null
            ? currentResult
            : {
              ...currentResult,
              songs_parsed: result.songs_parsed,
              songs: result.songs,
              duplicate_checksum_groups: result.duplicate_checksum_groups,
              disabled_song_conflicts: result.disabled_song_conflicts,
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

  const clearSongIniValidationError = useCallback(() => {
    setSongIniValidationError("");
    setSongIniConflictError("");
  }, []);

  const clearCompletedSongScan = useCallback(() => {
    setHasCompletedSongScan(false);
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
    setScanError("");
    setSongScanProgress(null);
    setIsInstrumentWizardOpen(false);
    setInstrumentAnalyzeStatus("idle");
    setInstrumentAnalyzeProgress(null);
    setInstrumentAnalyzeResult(null);
    setInstrumentAnalyzeError("");
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
    setIsScanWizardOpen(false);
    setHasCompletedSongScan(false);
    setScanStatus("idle");
    setScanError("");
    setSongScanProgress(null);
    setSongIniScanResult(null);
    setRepairedSongIniPaths([]);
    setDisabledSongIniPaths([]);
    setDeletedSongIniConflictPaths([]);
    setSongIniValidationError("");
    setSongIniConflictError("");
  }, []);

  const openInstrumentAnalyzer = useCallback(() => {
    setIsInstrumentWizardOpen(true);
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
    cancelScan,
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
    dismissScanToast,
    hasCompletedSongScan,
    instrumentAnalyzeError,
    instrumentAnalyzeProgress,
    instrumentAnalyzeResult,
    instrumentAnalyzeStatus,
    isInstrumentWizardOpen,
    isScanWizardOpen,
    openInstrumentAnalyzer,
    repairedSongIniPaths,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    songIniConflictError,
    songIniScanResult,
    songScanProgress,
    songIniValidationError,
    validateSongIni,
  };
}
