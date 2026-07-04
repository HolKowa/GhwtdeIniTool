import { useEffect } from "react";

import { CleanModsWizard } from "./components/CleanModsWizard";
import { InstrumentAnalyzeWizard } from "./components/InstrumentAnalyzeWizard";
import { ScanToast } from "./components/ScanToast";
import { ScanWizard } from "./components/ScanWizard";
import { ScannedSongsTable } from "./components/ScannedSongsTable";
import { SettingsDialog } from "./components/SettingsDialog";
import { UpdateDialog } from "./components/UpdateDialog";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useModsCleaner } from "./hooks/useModsCleaner";
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
    chooseModsFolder,
    isSettingsOpen,
    loadSettings,
    setKeepOriginalSongIni,
    setIsSettingsOpen,
    settings,
    settingsError,
    settingsStatus,
  } = useProjectSettings();
  const {
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
  } = useModsCleaner();
  const {
    analyzeInstruments,
    cancelInstrumentAnalyzer,
    cancelScan,
    clearCompletedSongScan,
    clearSongIniValidationError,
    closeInstrumentAnalyzer,
    confirmScan,
    copyContentIssuePath,
    deleteSongIniConflict,
    deletedSongIniConflictPaths,
    disableSongIni,
    disabledSongIniPaths,
    dismissScanToast,
    hasCompletedSongScan,
    instrumentAnalyzeError,
    instrumentAnalyzeProgress,
    instrumentAnalyzeResult,
    instrumentAnalyzeStatus,
    isInstrumentWizardOpen,
    isScanWizardOpen,
    openInstrumentAnalyzer,
    repairedSongIniPaths,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    songIniConflictError,
    songIniScanResult,
    songScanProgress,
    songIniValidationError,
    validateSongIni,
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
  const isScanBusy =
    scanStatus === "scanningSongs" ||
    scanStatus === "validatingSong" ||
    scanStatus === "disablingSong" ||
    scanStatus === "deletingSongConflict";
  const isCleanBusy =
    cleanStatus === "previewing" || cleanStatus === "deleting";
  const isInstrumentAnalyzeBusy = instrumentAnalyzeStatus === "analyzing";
  const isWorkflowOpen =
    isCleanWizardOpen || isScanWizardOpen || isInstrumentWizardOpen;
  const hasScannedSongs = hasCompletedSongScan && Boolean(songIniScanResult);
  const canAnalyzeInstruments =
    hasScannedSongs && !isWorkflowOpen;
  const activeToast = cleanToast ?? scanToast;
  const dismissActiveToast = cleanToast
    ? dismissCleanToast
    : dismissScanToast;
  const confirmCleanAndClearScan = async () => {
    const filesDeleted = await confirmClean();

    if (filesDeleted > 0) {
      clearCompletedSongScan();
    }
  };

  return (
    <main className="container">
      <div className="scan-panel">
        <button
          className="clean-button"
          type="button"
          onClick={cleanMods}
          disabled={isWorkflowOpen || isScanBusy || isCleanBusy}
        >
          {isCleanBusy ? "Cleaning..." : "Clean MODS folder"}
        </button>
        <button
          className="scan-button"
          type="button"
          onClick={scanMods}
          disabled={isWorkflowOpen || isScanBusy || isCleanBusy}
        >
          {isScanBusy ? "Scanning..." : "Scan MODS folder"}
        </button>
        {hasScannedSongs && (
          <button
            className="instrument-button"
            type="button"
            onClick={openInstrumentAnalyzer}
            disabled={
              !canAnalyzeInstruments ||
              isScanBusy ||
              isCleanBusy ||
              isInstrumentAnalyzeBusy
            }
          >
            {isInstrumentAnalyzeBusy ? "Analyzing..." : "Analyze instruments"}
          </button>
        )}
      </div>

      {hasScannedSongs && songIniScanResult && (
        <ScannedSongsTable
          onCopyFolderPath={copyContentIssuePath}
          songs={songIniScanResult.songs}
        />
      )}

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
          onChangeFolder={chooseModsFolder}
          onClose={() => {
            if (canCloseSettings) {
              setIsSettingsOpen(false);
            }
          }}
          onKeepOriginalSongIniChange={setKeepOriginalSongIni}
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

      {isCleanWizardOpen && (
        <CleanModsWizard
          cleanError={cleanError}
          cleanStatus={cleanStatus}
          deletePreview={deletePreview}
          onBack={backToCleanPattern}
          onCancel={cancelClean}
          onConfirmDelete={confirmCleanAndClearScan}
          onPreview={previewClean}
        />
      )}

      {isScanWizardOpen && (
        <ScanWizard
          onCancel={cancelScan}
          onConfirm={confirmScan}
          onCopyContentIssuePath={copyContentIssuePath}
          onDeleteSongIniConflict={deleteSongIniConflict}
          onDisableSongIni={disableSongIni}
          onSelectSongIni={clearSongIniValidationError}
          onValidateSongIni={validateSongIni}
          deletedSongIniConflictPaths={deletedSongIniConflictPaths}
          disabledSongIniPaths={disabledSongIniPaths}
          repairedSongIniPaths={repairedSongIniPaths}
          scanError={scanError}
          scanStatus={scanStatus}
          songIniConflictError={songIniConflictError}
          songIniScanResult={songIniScanResult}
          songScanProgress={songScanProgress}
          songIniValidationError={songIniValidationError}
        />
      )}

      {isInstrumentWizardOpen && (
        <InstrumentAnalyzeWizard
          error={instrumentAnalyzeError}
          onAnalyze={analyzeInstruments}
          onCancel={cancelInstrumentAnalyzer}
          onClose={closeInstrumentAnalyzer}
          progress={instrumentAnalyzeProgress}
          result={instrumentAnalyzeResult}
          status={instrumentAnalyzeStatus}
        />
      )}

      {activeToast && (
        <ScanToast onDismiss={dismissActiveToast} toast={activeToast} />
      )}
    </main>
  );
}

export default App;
