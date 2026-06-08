import { useCallback, useState } from "react";

import { scanModsFolder } from "../services/scanModsApi";
import type { ScanModsResult } from "../types/scanMods";

export type ScanModsStatus = "idle" | "scanning" | "complete" | "error";

export function useModsScanner() {
  const [scanStatus, setScanStatus] = useState<ScanModsStatus>("idle");
  const [scanResult, setScanResult] = useState<ScanModsResult | null>(null);
  const [scanError, setScanError] = useState("");

  const scanMods = useCallback(async () => {
    setScanStatus("scanning");
    setScanError("");

    try {
      const result = await scanModsFolder();
      setScanResult(result);
      setScanStatus("complete");
    } catch (err) {
      setScanResult(null);
      setScanError(String(err));
      setScanStatus("error");
    }
  }, []);

  return {
    scanError,
    scanMods,
    scanResult,
    scanStatus,
  };
}
