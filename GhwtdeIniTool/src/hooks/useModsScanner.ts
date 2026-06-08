import { useCallback, useEffect, useState } from "react";

import { previewScanModsFolder, scanModsFolder } from "../services/scanModsApi";
import type { ScanModsPreview, ScanModsResult } from "../types/scanMods";

export type ScanModsStatus = "idle" | "previewing" | "ready" | "moving" | "error";

export type ScanToast = {
  message: string;
  tone: "success" | "error";
};

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [scanPreview, setScanPreview] = useState<ScanModsPreview | null>(null);
  const [scanResult, setScanResult] = useState<ScanModsResult | null>(null);
  const [scanError, setScanError] = useState("");
  const [isScanWizardOpen, setIsScanWizardOpen] = useState(false);
  const [scanToast, setScanToast] = useState<ScanToast | null>(null);

  const scanMods = useCallback(async () => {
    setIsScanWizardOpen(true);
    setScanStatus("previewing");
    setScanError("");
    setScanPreview(null);
    setScanToast(null);

    try {
      const preview = await previewScanModsFolder();
      setScanPreview(preview);
      setScanStatus("ready");
    } catch (err) {
      setScanPreview(null);
      setScanResult(null);
      setScanError(String(err));
      setScanStatus("error");
    }
  }, []);

  const confirmScan = useCallback(async () => {
    setScanStatus("moving");
    setScanError("");

    try {
      const result = await scanModsFolder();
      setScanResult(result);
      setIsScanWizardOpen(false);
      setScanStatus("idle");
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
      setScanError(String(err));
      setScanStatus("error");
      setScanToast({
        message: String(err),
        tone: "error",
      });
    }
  }, []);

  const cancelScan = useCallback(() => {
    setIsScanWizardOpen(false);
    setScanStatus("idle");
    setScanError("");
    setScanPreview(null);
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
