import { useCallback, useState } from "react";

import { openModsFolderDialog } from "../services/modsFolderDialog";
import {
  loadProjectSettings,
  saveProjectSettings,
} from "../services/projectSettingsApi";
import type { ProjectSettings } from "../types/projectSettings";

export type SettingsStatus = "loading" | "ready" | "needs-folder" | "error";

const defaultSettings: ProjectSettings = {
  mods_dir: null,
  mods_dir_available: false,
  keep_original_song_ini: true,
  settings_file: "",
};

export function useProjectSettings() {
  const [settings, setSettings] = useState<ProjectSettings | null>(null);
  const [settingsStatus, setSettingsStatus] =
    useState<SettingsStatus>("loading");
  const [settingsError, setSettingsError] = useState("");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const persistSettings = useCallback(
    async (changes: Partial<ProjectSettings>) => {
      const savedSettings = await saveProjectSettings({
        ...defaultSettings,
        ...settings,
        ...changes,
      });

      setSettings(savedSettings);
      setSettingsStatus("ready");
      setSettingsError("");

      return savedSettings;
    },
    [settings],
  );

  const chooseModsFolder = useCallback(async () => {
    try {
      const selected = await openModsFolderDialog();

      if (typeof selected !== "string") {
        return;
      }

      await persistSettings({ mods_dir: selected });
      setIsSettingsOpen(true);
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, [persistSettings]);

  const setKeepOriginalSongIni = useCallback(
    async (keepOriginalSongIni: boolean) => {
      try {
        await persistSettings({
          keep_original_song_ini: keepOriginalSongIni,
        });
      } catch (err) {
        setSettingsStatus("error");
        setSettingsError(String(err));
      }
    },
    [persistSettings],
  );

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
  }, []);

  return {
    chooseModsFolder,
    isSettingsOpen,
    loadSettings,
    setKeepOriginalSongIni,
    setIsSettingsOpen,
    settings,
    settingsError,
    settingsStatus,
  };
}
