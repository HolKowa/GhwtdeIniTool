import { useCallback, useEffect, useState } from "react";

import { restoreAllOriginalSongIni } from "../services/scanModsApi";
import type { RestoreOriginalSongIniMode } from "../types/scanMods";

export type RestoreIniStatus = "idle" | "selecting" | "restoring" | "error";

export type RestoreIniToast = {
  message: string;
  tone: "success" | "error";
};

export function useIniRestorer() {
  const [restoreStatus, setRestoreStatus] =
    useState<RestoreIniStatus>("idle");
  const [restoreError, setRestoreError] = useState("");
  const [restoreErrors, setRestoreErrors] = useState<string[]>([]);
  const [isRestoreWizardOpen, setIsRestoreWizardOpen] = useState(false);
  const [restoreToast, setRestoreToast] = useState<RestoreIniToast | null>(
    null,
  );

  const openRestoreWizard = useCallback(() => {
    setIsRestoreWizardOpen(true);
    setRestoreStatus("selecting");
    setRestoreError("");
    setRestoreErrors([]);
    setRestoreToast(null);
  }, []);

  const confirmRestore = useCallback(
    async (mode: RestoreOriginalSongIniMode) => {
      if (restoreStatus !== "selecting") {
        return 0;
      }

      setRestoreStatus("restoring");
      setRestoreError("");
      setRestoreErrors([]);
      setRestoreToast(null);

      try {
        const result = await restoreAllOriginalSongIni(mode);
        const errorSummary = result.errors.length
          ? ` | ${result.errors.length} restore errors`
          : "";
        const message = `${result.files_restored} song.ini files restored${errorSummary}`;

        if (result.errors.length) {
          setIsRestoreWizardOpen(true);
          setRestoreStatus("error");
          setRestoreError(message);
          setRestoreErrors(result.errors);
          setRestoreToast({
            message,
            tone: "error",
          });
          return result.files_restored;
        }

        setIsRestoreWizardOpen(false);
        setRestoreStatus("idle");
        setRestoreToast({
          message,
          tone: "success",
        });
        return result.files_restored;
      } catch (err) {
        const message = String(err);

        setRestoreError(message);
        setRestoreErrors([]);
        setRestoreStatus("error");
        setRestoreToast({
          message,
          tone: "error",
        });
        return 0;
      }
    },
    [restoreStatus],
  );

  const cancelRestore = useCallback(() => {
    setIsRestoreWizardOpen(false);
    setRestoreStatus("idle");
    setRestoreError("");
    setRestoreErrors([]);
  }, []);

  const dismissRestoreToast = useCallback(() => {
    setRestoreToast(null);
  }, []);

  useEffect(() => {
    if (!restoreToast) {
      return;
    }

    const timeout = window.setTimeout(() => {
      setRestoreToast(null);
    }, 5000);

    return () => window.clearTimeout(timeout);
  }, [restoreToast]);

  return {
    cancelRestore,
    confirmRestore,
    dismissRestoreToast,
    isRestoreWizardOpen,
    openRestoreWizard,
    restoreError,
    restoreErrors,
    restoreStatus,
    restoreToast,
  };
}
