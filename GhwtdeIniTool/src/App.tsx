import { useEffect } from "react";

import { SettingsDialog } from "./components/SettingsDialog";
import { UpdateDialog } from "./components/UpdateDialog";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useModsScanner } from "./hooks/useModsScanner";
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
  const { scanError, scanMods, scanResult, scanStatus } = useModsScanner();

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
  const scanSummary = scanResult
    ? [
        `${scanResult.categories_found} categories found`,
        `${scanResult.category_folders_moved} folders moved`,
        `${scanResult.files_moved} files moved`,
        `${scanResult.renamed_destinations} renamed`,
        `${scanResult.errors.length} errors`,
      ].join(" | ")
    : "";

  return (
    <main className="container">
      <div className="scan-panel">
        <button
          className="scan-button"
          type="button"
          onClick={scanMods}
          disabled={scanStatus === "scanning"}
        >
          {scanStatus === "scanning" ? "Scanning..." : "Scan MODS folder"}
        </button>

        {(scanStatus === "complete" || scanStatus === "error") && (
          <div
            className={`scan-summary ${
              scanStatus === "error" || scanResult?.errors.length
                ? "scan-summary-error"
                : ""
            }`}
            role="status"
          >
            {scanStatus === "error" ? (
              <p>{scanError}</p>
            ) : (
              <>
                <p>{scanSummary}</p>
                {scanResult && !scanResult.moved_categories_enabled && (
                  <p>Category moving is disabled because no extra folder is available.</p>
                )}
                {scanResult?.errors.map((error, index) => (
                  <p key={`${index}-${error}`}>{error}</p>
                ))}
              </>
            )}
          </div>
        )}
      </div>

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
