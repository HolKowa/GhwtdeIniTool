import { useCallback, useEffect, useState } from "react";

import {
  deleteKeepOnlyFiles,
  previewKeepOnlyFilesDelete,
  previewScanModsFolder,
  scanModsFolder,
} from "../services/scanModsApi";
import type {
  DeleteFilesPreview,
  DeleteFilesResult,
  ScanModsPreview,
  ScanModsResult,
} from "../types/scanMods";

export type ScanModsStatus =
  | "idle"
  | "previewing"
  | "readyMove"
  | "moving"
  | "readyDelete"
  | "deleting"
  | "error";

export type ScanToast = {
  message: string;
  tone: "success" | "error";
};

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [scanPreview, setScanPreview] = useState<ScanModsPreview | null>(null);
  const [scanResult, setScanResult] = useState<ScanModsResult | null>(null);
  const [deletePreview, setDeletePreview] =
    useState<DeleteFilesPreview | null>(null);
  const [deleteResult, setDeleteResult] = useState<DeleteFilesResult | null>(
    null,
  );
  const [scanError, setScanError] = useState("");
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
  const [scanToast, setScanToast] = useState<ScanToast | null>(null);

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setScanStatus("previewing");
    setScanError("");
    setScanPreview(null);
    setScanResult(null);
    setDeletePreview(null);
    setDeleteResult(null);
    setScanToast(null);

    try {
      const preview = await previewScanModsFolder();
      setScanPreview(preview);
      setScanStatus("readyMove");
    } catch (err) {
      setScanPreview(null);
      setScanResult(null);
      setDeletePreview(null);
      setDeleteResult(null);
      setScanError(String(err));
      setScanStatus("error");
    }
  }, []);

  const confirmScan = useCallback(async () => {
    if (scanStatus === "readyDelete") {
      setScanStatus("deleting");
      setScanError("");

      try {
        const result = await deleteKeepOnlyFiles(
          deletePreview?.files_to_delete ?? [],
        );
        setDeleteResult(result);
        setIsScanWizardOpen(false);
        setScanStatus("idle");
        setScanToast({
          message: result.errors.length
            ? `${result.files_deleted} files deleted | ${result.errors.length} errors`
            : `${result.files_deleted} files deleted`,
          tone: result.errors.length ? "error" : "success",
        });
      } catch (err) {
        setDeleteResult(null);
        setScanError(String(err));
        setScanStatus("error");
        setScanToast({
          message: String(err),
          tone: "error",
        });
      }

      return;
    }

    setScanStatus("moving");
    setScanError("");

    try {
      const result = await scanModsFolder();
      setScanResult(result);
      const nextDeletePreview = await previewKeepOnlyFilesDelete();
      setDeletePreview(nextDeletePreview);
      setDeleteResult(null);
      setScanStatus("readyDelete");
      setScanToast({
        message: [
          `${result.category_folders_moved} folders moved`,
          `${result.files_moved} files moved`,
          `${result.renamed_destinations} renamed`,
          `${result.errors.length} errors`,
        ].join(" | "),
        tone: result.errors.length ? "error" : "success",
      });
    } catch (err) {
      setScanResult(null);
      setDeletePreview(null);
      setDeleteResult(null);
      setScanError(String(err));
      setScanStatus("error");
      setScanToast({
        message: String(err),
        tone: "error",
      });
    }
  }, [deletePreview, scanResult, scanStatus]);

  const cancelScan = useCallback(() => {
    setIsScanWizardOpen(false);
    setScanStatus("idle");
    setScanError("");
    setScanPreview(null);
    setScanResult(null);
    setDeletePreview(null);
    setDeleteResult(null);
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
    confirmScan,
    deletePreview,
    deleteResult,
    dismissScanToast,
    isScanWizardOpen,
    scanError,
    scanMods,
    scanPreview,
    scanResult,
    scanStatus,
    scanToast,
  };
}
