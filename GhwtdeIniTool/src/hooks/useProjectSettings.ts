import { useCallback, useState } from "react";

import { openModsFolderDialog } from "../services/modsFolderDialog";
import {
  loadProjectSettings,
  saveProjectSettings,
} from "../services/projectSettingsApi";
import type { ProjectSettings } from "../types/projectSettings";

export type SettingsStatus = "loading" | "ready" | "needs-folder" | "error";

export function useProjectSettings() {
  const [settings, setSettings] = useState<ProjectSettings | null>(null);
  const [settingsStatus, setSettingsStatus] =
    useState<SettingsStatus>("loading");
  const [settingsError, setSettingsError] = useState("");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const chooseModsFolder = useCallback(async () => {
    try {
      const selected = await openModsFolderDialog();

      if (typeof selected !== "string") {
        return;
      }

      const savedSettings = await saveProjectSettings(selected);

      setSettings(savedSettings);
      setSettingsStatus("ready");
      setIsSettingsOpen(true);
      setSettingsError("");
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, []);

  const loadSettings = useCallback(async () => {
    try {
      const loadedSettings = await loadProjectSettings();
      setSettings(loadedSettings);

      if (!loadedSettings.mods_dir || !loadedSettings.mods_dir_available) {
        setSettingsStatus("needs-folder");
        setIsSettingsOpen(true);
        return;
      }

      setSettingsStatus("ready");
      setSettingsError("");
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, [chooseModsFolder]);

  return {
    chooseModsFolder,
    isSettingsOpen,
    loadSettings,
    setIsSettingsOpen,
    settings,
    settingsError,
    settingsStatus,
  };
}
