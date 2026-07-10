import { useEffect } from "react";

import { CleanModsWizard } from "./components/CleanModsWizard";
import { InstrumentAnalyzeWizard } from "./components/InstrumentAnalyzeWizard";
import { RestoreIniWizard } from "./components/RestoreIniWizard";
import { ScanToast } from "./components/ScanToast";
import { ScanWizard } from "./components/ScanWizard";
import { ScannedSongsTable } from "./components/ScannedSongsTable";
import { SettingsDialog } from "./components/SettingsDialog";
import { UpdateDialog } from "./components/UpdateDialog";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useModsCleaner } from "./hooks/useModsCleaner";
import { useModsScanner } from "./hooks/useModsScanner";
import { useIniRestorer } from "./hooks/useIniRestorer";
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
    cancelRestore,
    confirmRestore,
    dismissRestoreToast,
    isRestoreWizardOpen,
    openRestoreWizard,
    restoreError,
    restoreErrors,
    restoreStatus,
    restoreToast,
  } = useIniRestorer();
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
    enableSongIni,
    dismissScanToast,
    hasCompletedSongScan,
    includedSongPaths,
    instrumentAnalyzeError,
    instrumentAnalyzeProgress,
    instrumentAnalyzeResult,
    instrumentAnalyzeStatus,
    isInstrumentWizardOpen,
    isScanWizardConflictsOnly,
    isScanWizardOpen,
    openInstrumentAnalyzer,
    repairedSongIniPaths,
    restoringScannedSongPaths,
    restoreScannedSongOriginal,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    saveScannedSongMetadata,
    savingScannedSongPaths,
    songIniConflictError,
    songIniScanResult,
    songScanProgress,
    songIniValidationError,
    setSongIncluded,
    setSongsIncluded,
    validateSongIni,
    verifiedContentIssueSongPaths,
    verifyingContentIssueSongPath,
    verifyContentIssueSong,
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
    scanStatus === "verifyingSong" ||
    scanStatus === "disablingSong" ||
    scanStatus === "deletingSongConflict";
  const isCleanBusy =
    cleanStatus === "previewing" || cleanStatus === "deleting";
  const isRestoreBusy = restoreStatus === "restoring";
  const isInstrumentAnalyzeBusy = instrumentAnalyzeStatus === "analyzing";
  const isWorkflowOpen =
    isRestoreWizardOpen ||
    isCleanWizardOpen ||
    isScanWizardOpen ||
    isInstrumentWizardOpen;
  const hasScannedSongs = hasCompletedSongScan && Boolean(songIniScanResult);
  const canAnalyzeInstruments =
    hasScannedSongs && !isWorkflowOpen;
  const activeToast = restoreToast ?? cleanToast ?? scanToast;
  const dismissActiveToast = restoreToast
    ? dismissRestoreToast
    : cleanToast
      ? dismissCleanToast
      : dismissScanToast;
  const confirmCleanAndClearScan = async () => {
    const filesDeleted = await confirmClean();

    if (filesDeleted > 0) {
      clearCompletedSongScan();
    }
  };
  const confirmRestoreAndClearScan = async (
    mode: Parameters<typeof confirmRestore>[0],
  ) => {
    const filesRestored = await confirmRestore(mode);

    if (filesRestored > 0) {
      clearCompletedSongScan();
    }
  };

  return (
    <main className="container">
      <div className="scan-panel">
        <button
          className="restore-button"
          type="button"
          onClick={openRestoreWizard}
          disabled={isWorkflowOpen || isScanBusy || isCleanBusy || isRestoreBusy}
        >
          {isRestoreBusy ? "Restoring..." : "Restore INI files"}
        </button>
        <button
          className="clean-button"
          type="button"
          onClick={cleanMods}
          disabled={isWorkflowOpen || isScanBusy || isCleanBusy || isRestoreBusy}
        >
          {isCleanBusy ? "Cleaning..." : "Clean MODS folder"}
        </button>
        <button
          className="scan-button"
          type="button"
          onClick={scanMods}
          disabled={isWorkflowOpen || isScanBusy || isCleanBusy || isRestoreBusy}
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
              isRestoreBusy ||
              isInstrumentAnalyzeBusy
            }
          >
            {isInstrumentAnalyzeBusy ? "Analyzing..." : "Analyze instruments"}
          </button>
        )}
        {hasScannedSongs && (
          <button
            className="categorize-button"
            type="button"
            disabled
            title="Backend categorization is not implemented yet"
          >
            Categorize
          </button>
        )}
      </div>

      {hasScannedSongs && songIniScanResult && (
        <ScannedSongsTable
          duplicateChecksumGroups={songIniScanResult.duplicate_checksum_groups}
          includedSongPaths={includedSongPaths}
          onCopyFolderPath={copyContentIssuePath}
          onRestoreOriginal={restoreScannedSongOriginal}
          onSaveMetadata={saveScannedSongMetadata}
          onSetSongIncluded={setSongIncluded}
          onSetSongsIncluded={setSongsIncluded}
          restoringSongPaths={restoringScannedSongPaths}
          savingSongPaths={savingScannedSongPaths}
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

      {isRestoreWizardOpen && (
        <RestoreIniWizard
          error={restoreError}
          errors={restoreErrors}
          onCancel={cancelRestore}
          onConfirm={confirmRestoreAndClearScan}
          status={restoreStatus}
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
          onEnableSongIni={enableSongIni}
          onExcludeScannedSong={(relativePath) =>
            setSongIncluded(relativePath, false)
          }
          onSelectSongIni={clearSongIniValidationError}
          onValidateSongIni={validateSongIni}
          onVerifyContentIssueSong={verifyContentIssueSong}
          deletedSongIniConflictPaths={deletedSongIniConflictPaths}
          disabledSongIniPaths={disabledSongIniPaths}
          includedSongPaths={includedSongPaths}
          isConflictsOnly={isScanWizardConflictsOnly}
          repairedSongIniPaths={repairedSongIniPaths}
          scanError={scanError}
          scanStatus={scanStatus}
          songIniConflictError={songIniConflictError}
          songIniScanResult={songIniScanResult}
          songScanProgress={songScanProgress}
          songIniValidationError={songIniValidationError}
          verifiedContentIssueSongPaths={verifiedContentIssueSongPaths}
          verifyingContentIssueSongPath={verifyingContentIssueSongPath}
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
