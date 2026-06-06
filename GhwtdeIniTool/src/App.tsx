import { useEffect } from "react";

import { SettingsDialog } from "./components/SettingsDialog";
import { UpdateDialog } from "./components/UpdateDialog";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useProjectSettings } from "./hooks/useProjectSettings";
import "./App.css";

function App() {
  const {
    checkForUpdates,
    downloadProgress,
    errorMessage,
    installUpdate,
    skipUpdate,
    startDownload,
    updateStatus,
  } = useAppUpdater();
  const {
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
  } = useProjectSettings();

  useEffect(() => {
    checkForUpdates();
    loadSettings();
  }, [checkForUpdates, loadSettings]);

  const shouldShowUpdatePopup =
    updateStatus === "available" ||
    updateStatus === "downloading" ||
    updateStatus === "ready" ||
    updateStatus === "error";
  const canCloseSettings = Boolean(settings?.mods_dir_available);

  return (
    <main className="container">
      <button
        className="settings-button"
        type="button"
        onClick={() => setIsSettingsOpen(true)}
        aria-label="Open settings"
      >
        Settings
      </button>

      {isSettingsOpen && (
        <SettingsDialog
          canClose={canCloseSettings}
          onChangeCategoriesExtraFolder={chooseCategoriesExtraFolder}
          onChangeFolder={chooseModsFolder}
          onClearCategoriesExtraFolder={clearCategoriesExtraFolder}
          onClose={() => {
            if (canCloseSettings) {
              setIsSettingsOpen(false);
            }
          }}
          onKeepOnlyFilesPatternChange={setKeepOnlyFilesPattern}
          onKeepOnlyFilesWithPatternChange={setKeepOnlyFilesWithPattern}
          settings={settings}
          settingsError={settingsError}
          settingsStatus={settingsStatus}
        />
      )}

      {shouldShowUpdatePopup && (
        <UpdateDialog
          downloadProgress={downloadProgress}
          errorMessage={errorMessage}
          onInstall={installUpdate}
          onSkip={skipUpdate}
          onStartDownload={startDownload}
          updateStatus={updateStatus}
        />
      )}
    </main>
  );
}

export default App;
