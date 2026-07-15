import { useCallback, useEffect, useState } from "react";

import {
  deleteKeepOnlyFiles,
  previewKeepOnlyFilesDelete,
} from "../services/scanModsApi";
import type { DeleteFilesPreview } from "../types/scanMods";

export type CleanModsStatus =
  | "idle"
  | "editing"
  | "previewing"
  | "readyDelete"
  | "deleting"
  | "error";

export type CleanToast = {
  message: string;
  tone: "success" | "error";
};

export function useModsCleaner() {
  const [cleanStatus, setCleanStatus] = useState<CleanModsStatus>("idle");
  const [deletePreview, setDeletePreview] =
    useState<DeleteFilesPreview | null>(null);
  const [cleanError, setCleanError] = useState("");
  const [isCleanWizardOpen, setIsCleanWizardOpen] = useState(false);
  const [cleanToast, setCleanToast] = useState<CleanToast | null>(null);

  const cleanMods = useCallback(() => {
    setIsCleanWizardOpen(true);
    setCleanStatus("editing");
    setCleanError("");
    setDeletePreview(null);
    setCleanToast(null);
  }, []);

  const previewClean = useCallback(async (keepOnlyFilesPattern: string) => {
    setCleanStatus("previewing");
    setCleanError("");
    setDeletePreview(null);
    setCleanToast(null);

    try {
      const preview = await previewKeepOnlyFilesDelete(keepOnlyFilesPattern);
      setDeletePreview(preview);
      setCleanStatus("readyDelete");
    } catch (err) {
      setCleanError(String(err));
      setCleanStatus("error");
    }
  }, []);

  const confirmClean = useCallback(async (filesToDelete: string[]) => {
    if (cleanStatus !== "readyDelete") {
      return 0;
    }

    setCleanStatus("deleting");
    setCleanError("");

    try {
      const result = await deleteKeepOnlyFiles(
        filesToDelete,
      );
      const errorSummary = result.errors.length
        ? ` | ${result.errors.length} delete errors`
        : "";

      setIsCleanWizardOpen(false);
      setCleanStatus("idle");
      setDeletePreview(null);
      setCleanToast({
        message: `${result.files_deleted} files deleted${errorSummary}`,
        tone: result.errors.length ? "error" : "success",
      });
      return result.files_deleted;
    } catch (err) {
      setCleanError(String(err));
      setCleanStatus("error");
      setCleanToast({
        message: String(err),
        tone: "error",
      });
      return 0;
    }
  }, [cleanStatus, deletePreview]);

  const backToCleanPattern = useCallback(() => {
    if (cleanStatus !== "readyDelete") {
      return;
    }

    setCleanStatus("editing");
    setCleanError("");
    setDeletePreview(null);
  }, [cleanStatus]);

  const cancelClean = useCallback(() => {
    setIsCleanWizardOpen(false);
    setCleanStatus("idle");
    setCleanError("");
    setDeletePreview(null);
  }, []);

  const dismissCleanToast = useCallback(() => {
    setCleanToast(null);
  }, []);

  useEffect(() => {
    if (!cleanToast) {
      return;
    }

    const timeout = window.setTimeout(() => {
      setCleanToast(null);
    }, 5000);

    return () => window.clearTimeout(timeout);
  }, [cleanToast]);

  return {
    backToCleanPattern,
    cancelClean,
    cleanError,
    cleanMods,
    cleanStatus,
    cleanToast,
    confirmClean,
    deletePreview,
    dismissCleanToast,
    isCleanWizardOpen,
    previewClean,
  };
}
