import { useCallback, useState } from "react";

import {
  openGameLogosFolderDialog,
  openModsFolderDialog,
} from "../services/modsFolderDialog";
import {
  loadProjectSettings,
  saveProjectSettings,
} from "../services/projectSettingsApi";
import type { ProjectSettings } from "../types/projectSettings";

export type SettingsStatus = "loading" | "ready" | "needs-folder" | "error";

const defaultSettings: ProjectSettings = {
  disclaimer_accepted: false,
  mods_dir: null,
  mods_dir_available: false,
  official_gamelogos_dir: null,
  official_gamelogos_dir_available: false,
  keep_original_song_ini: true,
  check_for_updates_on_startup: true,
  settings_file: "",
};

function hasCompleteSetup(settings: ProjectSettings) {
  return (
    settings.disclaimer_accepted &&
    Boolean(settings.mods_dir && settings.mods_dir_available) &&
    Boolean(
      settings.official_gamelogos_dir &&
        settings.official_gamelogos_dir_available,
    )
  );
}

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

  const chooseGameLogosFolder = useCallback(async () => {
    try {
      const selected = await openGameLogosFolderDialog();

      if (typeof selected !== "string") {
        return;
      }

      await persistSettings({ official_gamelogos_dir: selected });
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

  const setCheckForUpdatesOnStartup = useCallback(
    async (checkForUpdatesOnStartup: boolean) => {
      try {
        await persistSettings({
          check_for_updates_on_startup: checkForUpdatesOnStartup,
        });
        return true;
      } catch (err) {
        setSettingsStatus("error");
        setSettingsError(String(err));
        return false;
      }
    },
    [persistSettings],
  );

  const acceptDisclaimer = useCallback(async () => {
    try {
      const savedSettings = await persistSettings({
        disclaimer_accepted: true,
      });

      if (hasCompleteSetup(savedSettings)) {
        setIsSettingsOpen(false);
      }
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
    }
  }, [persistSettings]);

  const loadSettings = useCallback(async () => {
    try {
      const loadedSettings = await loadProjectSettings();
      setSettings(loadedSettings);

      if (!hasCompleteSetup(loadedSettings)) {
        setSettingsStatus("needs-folder");
        setIsSettingsOpen(true);
        return loadedSettings;
      }

      setSettingsStatus("ready");
      setSettingsError("");
      return loadedSettings;
    } catch (err) {
      setSettingsStatus("error");
      setSettingsError(String(err));
      return null;
    }
  }, []);

  return {
    acceptDisclaimer,
    chooseGameLogosFolder,
    chooseModsFolder,
    isSettingsOpen,
    loadSettings,
    setCheckForUpdatesOnStartup,
    setKeepOriginalSongIni,
    setIsSettingsOpen,
    settings,
    settingsError,
    settingsStatus,
  };
}
