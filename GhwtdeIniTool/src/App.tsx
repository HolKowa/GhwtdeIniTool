import { useEffect } from "react";

import { ScanToast } from "./components/ScanToast";
import { ScanWizard } from "./components/ScanWizard";
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
    settings,
    settingsError,
    settingsStatus,
  } = useProjectSettings();
  const {
    cancelScan,
    confirmScan,
    dismissScanToast,
    isScanWizardOpen,
    scanError,
    scanMods,
    scanPreview,
    scanStatus,
    scanToast,
  } = useModsScanner();

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
      <div className="scan-panel">
        <button
          className="scan-button"
          type="button"
          onClick={scanMods}
          disabled={scanStatus === "previewing" || scanStatus === "moving"}
        >
          {scanStatus === "previewing" ? "Scanning..." : "Scan MODS folder"}
        </button>
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

      {isScanWizardOpen && (
        <ScanWizard
          onCancel={cancelScan}
          onConfirm={confirmScan}
          preview={scanPreview}
          scanError={scanError}
          scanStatus={scanStatus}
        />
      )}

      {scanToast && (
        <ScanToast onDismiss={dismissScanToast} toast={scanToast} />
      )}
    </main>
  );
}

export default App;
