import { useCallback, useRef, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "none"
  | "error";

export function useAppUpdater() {
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("checking");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [errorMessage, setErrorMessage] = useState("");
  const updateRef = useRef<Update | null>(null);

  const checkForUpdates = useCallback(async () => {
    try {
      const update = await check();
      if (!update) {
        setUpdateStatus("none");
        return;
      }

      updateRef.current = update;
      setUpdateStatus("available");
    } catch {
      updateRef.current = null;
      setUpdateStatus("none");
    }
  }, []);

  const startDownload = useCallback(async () => {
    const update = updateRef.current;
    if (!update) {
      return;
    }

    setUpdateStatus("downloading");
    setDownloadProgress(0);
    let downloaded = 0;
    let contentLength = 0;

    try {
      await update.download((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setDownloadProgress(Math.round((downloaded / contentLength) * 100));
            }
            break;
          case "Finished":
            setDownloadProgress(100);
            break;
        }
      });

      setUpdateStatus("ready");
    } catch (err) {
      setUpdateStatus("error");
      setErrorMessage(String(err));
    }
  }, []);

  const installUpdate = useCallback(async () => {
    try {
      const update = updateRef.current;
      if (update) {
        await update.install();
        await relaunch();
      }
    } catch (err) {
      setUpdateStatus("error");
      setErrorMessage(String(err));
    }
  }, []);

  const skipUpdate = useCallback(() => {
    updateRef.current = null;
    setUpdateStatus("none");
  }, []);

  return {
    checkForUpdates,
    downloadProgress,
    errorMessage,
    installUpdate,
    skipUpdate,
    startDownload,
    updateStatus,
  };
}
