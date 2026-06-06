import { useCallback, useState } from "react";

import {
  openCategoriesExtraFolderDialog,
  openModsFolderDialog,
} from "../services/modsFolderDialog";
import {
  loadProjectSettings,
  saveProjectSettings,
} from "../services/projectSettingsApi";
import type { ProjectSettings } from "../types/projectSettings";

export type SettingsStatus = "loading" | "ready" | "needs-folder" | "error";

const defaultSettings: ProjectSettings = {
  mods_dir: null,
  mods_dir_available: false,
  categories_extra_dir: null,
  categories_extra_dir_available: false,
  keep_only_files_with_pattern: false,
  keep_only_files_pattern: "song.ini,*_song.pak.xen,*.fsb.xen",
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

  const chooseCategoriesExtraFolder = useCallback(async () => {
    try {
      const selected = await openCategoriesExtraFolderDialog();

      if (typeof selected !== "string") {
        return;
      }

      await persistSettings({ categories_extra_dir: selected });
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, [persistSettings]);

  const clearCategoriesExtraFolder = useCallback(async () => {
    try {
      await persistSettings({ categories_extra_dir: null });
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, [persistSettings]);

  const setKeepOnlyFilesWithPattern = useCallback(
    async (keepOnlyFilesWithPattern: boolean) => {
      try {
        await persistSettings({
          keep_only_files_with_pattern: keepOnlyFilesWithPattern,
        });
      } catch (err) {
        setSettingsStatus("error");
        setSettingsError(String(err));
      }
    },
    [persistSettings],
  );

  const setKeepOnlyFilesPattern = useCallback(
    async (keepOnlyFilesPattern: string) => {
      try {
        await persistSettings({
          keep_only_files_pattern: keepOnlyFilesPattern,
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
    chooseCategoriesExtraFolder,
    chooseModsFolder,
    clearCategoriesExtraFolder,
    isSettingsOpen,
    loadSettings,
    setIsSettingsOpen,
    setKeepOnlyFilesPattern,
    setKeepOnlyFilesWithPattern,
    settings,
    settingsError,
    settingsStatus,
  };
}
