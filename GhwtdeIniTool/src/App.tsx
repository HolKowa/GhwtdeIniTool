import { useEffect, useState } from "react";

import { CleanModsWizard } from "./components/CleanModsWizard";
import { AboutDialog } from "./components/AboutDialog";
import { CategorizeWizard } from "./components/CategorizeWizard";
import { BackupWizard } from "./components/BackupWizard";
import { ExperimentalWarningDialog } from "./components/ExperimentalWarningDialog";
import { FixGameIconsWizard } from "./components/FixGameIconsWizard";
import { FixModsFolderWizard } from "./components/FixModsFolderWizard";
import { InstrumentAnalyzeWizard } from "./components/InstrumentAnalyzeWizard";
import { RestoreIniWizard } from "./components/RestoreIniWizard";
import { ScanToast } from "./components/ScanToast";
import { ScanWizard } from "./components/ScanWizard";
import {
  ScannedSongsTable,
  type CategorizationTableState,
} from "./components/ScannedSongsTable";
import { SettingsDialog } from "./components/SettingsDialog";
import { ThirdPartyLicensesDialog } from "./components/ThirdPartyLicensesDialog";
import { UpdateDialog } from "./components/UpdateDialog";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useModsCleaner } from "./hooks/useModsCleaner";
import { useModsScanner } from "./hooks/useModsScanner";
import { useIniRestorer } from "./hooks/useIniRestorer";
import { useProjectSettings } from "./hooks/useProjectSettings";
import type { ScannedSongMetadata } from "./types/scanMods";
import "./App.css";

function App() {
  const [isAboutOpen, setIsAboutOpen] = useState(false);
  const [isExperimentalWarningOpen, setIsExperimentalWarningOpen] =
    useState(false);
  const [isThirdPartyLicensesOpen, setIsThirdPartyLicensesOpen] =
    useState(false);
  const [isCategorizeWizardOpen, setIsCategorizeWizardOpen] = useState(false);
  const [isBackupWizardOpen, setIsBackupWizardOpen] = useState(false);
  const [backupToast, setBackupToast] = useState<{
    message: string;
    tone: "success" | "error";
  } | null>(null);
  const [draftMetadata, setDraftMetadata] = useState<Record<string, ScannedSongMetadata>>({});
  const [importedMetadataByPath, setImportedMetadataByPath] = useState<Record<string, ScannedSongMetadata>>({});
  const [categorizationTableState, setCategorizationTableState] =
    useState<CategorizationTableState>({
      hasActiveFilters: false,
      hasUnsavedSongChanges: false,
      orderedSongPaths: [],
      sortKey: "artist",
    });
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
    applyCategorization,
    applyGameIconSongFixRows,
    cancelInstrumentAnalyzer,
    cancelScan,
    categorizeSongsError,
    categorizeSongsStatus,
    clearCompletedSongScan,
    clearSongIniValidationError,
    closeInstrumentAnalyzer,
    closeOfficialCategories,
    confirmScan,
    copyContentIssuePath,
    deleteSongIniConflict,
    deletedSongIniConflictPaths,
    disableAllOfficialCategories,
    disableOfficialCategoryRow,
    disableSongIni,
    disabledSongIniPaths,
    enableOfficialCategoryRow,
    enableSongIni,
    fixGameIconCategoryFolders,
    folderSanitizeError,
    folderSanitizeResult,
    folderSanitizeStatus,
    dismissScanToast,
    gameIconCategoryError,
    gameIconCategoryResult,
    gameIconCategoryStatus,
    gameIconSongFixPreview,
    hasCompletedSongScan,
    includedSongPaths,
    instrumentAnalyzeError,
    instrumentAnalyzeProgress,
    instrumentAnalyzeResult,
    instrumentAnalyzeStatus,
    isGameIconWizardOpen,
    isInstrumentWizardOpen,
    isOfficialCategoryWizardOpen,
    isScanWizardConflictsOnly,
    isScanWizardOpen,
    closeGameIconCategories,
    openInstrumentAnalyzer,
    openGameIconCategories,
    openOfficialCategories,
    officialCategoryError,
    officialCategoryResult,
    officialCategoryStatus,
    previewGameIconSongFixRows,
    refreshOfficialCategories,
    refreshModsFolderNames,
    originalFaultySongIniFiles,
    repairedSongIniPaths,
    restoringScannedSongPaths,
    restoreScannedSongOriginal,
    scanError,
    scanMods,
    scanStatus,
    scanToast,
    saveScannedSongMetadata,
    sanitizeModsFolders,
    savingScannedSongPaths,
    songIniConflictError,
    songIniScanResult,
    songScanProgress,
    songIniValidationError,
    setSongIncluded,
    setDuplicateSongIncluded,
    setSongsIncluded,
    undoSongIni,
    validateSongIni,
    verifyContentIssueSongs,
  } = useModsScanner();

  useEffect(() => {
    void (async () => {
      const loadedSettings = await loadSettings();
      if (loadedSettings?.check_for_updates_on_startup) {
        await checkForUpdates();
      }
    })();
  }, [checkForUpdates, loadSettings]);

  const disableStartupUpdateChecks = async () => {
    if (await setCheckForUpdatesOnStartup(false)) {
      skipUpdate();
    }
  };

  const shouldShowUpdatePopup =
    updateStatus === "available" ||
    updateStatus === "downloading" ||
    updateStatus === "ready" ||
    updateStatus === "error";
  const canCloseSettings = Boolean(
    settings?.disclaimer_accepted &&
      settings.mods_dir_available &&
      settings.official_gamelogos_dir_available,
  );
  const isScanBusy =
    scanStatus === "scanningSongs" ||
    scanStatus === "validatingSong" ||
    scanStatus === "undoingSong" ||
    scanStatus === "verifyingSong" ||
    scanStatus === "disablingSong" ||
    scanStatus === "enablingSong" ||
    scanStatus === "deletingSongConflict";
  const isCleanBusy =
    cleanStatus === "previewing" || cleanStatus === "deleting";
  const isRestoreBusy = restoreStatus === "restoring";
  const isInstrumentAnalyzeBusy = instrumentAnalyzeStatus === "analyzing";
  const isGameIconBusy =
    gameIconCategoryStatus === "scanning" || gameIconCategoryStatus === "fixing";
  const isOfficialCategoryBusy = officialCategoryStatus === "scanning" || officialCategoryStatus === "disabling" || officialCategoryStatus === "enabling";
  const isWorkflowOpen =
    isRestoreWizardOpen ||
    isCleanWizardOpen ||
    isScanWizardOpen ||
    isInstrumentWizardOpen ||
    isGameIconWizardOpen ||
    isOfficialCategoryWizardOpen ||
    isCategorizeWizardOpen ||
    isBackupWizardOpen;
  const hasScannedSongs = hasCompletedSongScan && Boolean(songIniScanResult);
  const canAnalyzeInstruments =
    hasScannedSongs && !isWorkflowOpen;
  const canFixGameIcons = hasScannedSongs && !isWorkflowOpen;
  const canCategorize = hasScannedSongs && !isWorkflowOpen;
  const activeToast = restoreToast ?? cleanToast ?? scanToast ?? backupToast;
  const dismissActiveToast = restoreToast
    ? dismissRestoreToast
    : cleanToast
      ? dismissCleanToast
      : scanToast
        ? dismissScanToast
        : () => setBackupToast(null);

  useEffect(() => {
    if (!backupToast) return;
    const timeout = window.setTimeout(() => setBackupToast(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [backupToast]);
  const confirmCleanAndClearScan = async (filesToDelete: string[]) => {
    const filesDeleted = await confirmClean(filesToDelete);

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
          disabled={
            isWorkflowOpen ||
            isScanBusy ||
            isCleanBusy ||
            isRestoreBusy ||
            isGameIconBusy
          }
        >
          {isRestoreBusy ? "Restoring..." : "Restore INI files"}
        </button>
        <button
          className="clean-button"
          type="button"
          onClick={cleanMods}
          disabled={
            isWorkflowOpen ||
            isScanBusy ||
            isCleanBusy ||
            isRestoreBusy ||
            isGameIconBusy
          }
        >
          {isCleanBusy ? "Cleaning..." : "Clean MODS folder"}
        </button>
        <button
          className="game-icon-button"
          type="button"
          onClick={openOfficialCategories}
          disabled={
            isWorkflowOpen ||
            isScanBusy ||
            isCleanBusy ||
            isRestoreBusy ||
            isInstrumentAnalyzeBusy ||
            isGameIconBusy ||
            isOfficialCategoryBusy
          }
        >
          {isOfficialCategoryBusy ? "Verifying..." : "Fix MODS folder"}
        </button>
        <button
          className="scan-button"
          type="button"
          onClick={scanMods}
          disabled={
            isWorkflowOpen ||
            isScanBusy ||
            isCleanBusy ||
            isRestoreBusy ||
            isGameIconBusy
          }
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
              isInstrumentAnalyzeBusy ||
              isGameIconBusy
            }
          >
            {isInstrumentAnalyzeBusy ? "Analyzing..." : "Analyze instruments"}
          </button>
        )}
        {hasScannedSongs && (
          <button
            className="game-icon-button"
            type="button"
            onClick={openGameIconCategories}
            disabled={
              !canFixGameIcons ||
              isScanBusy ||
              isCleanBusy ||
              isRestoreBusy ||
              isInstrumentAnalyzeBusy ||
              isGameIconBusy
            }
          >
            {isGameIconBusy ? "Fixing..." : "Fix GameIcons"}
          </button>
        )}
        {hasScannedSongs && (
          <button
            className="categorize-button"
            type="button"
            onClick={() => setIsCategorizeWizardOpen(true)}
            disabled={
              !canCategorize ||
              isScanBusy ||
              isCleanBusy ||
              isRestoreBusy ||
              isInstrumentAnalyzeBusy ||
              isGameIconBusy
            }
          >
            Categorize
          </button>
        )}
        {hasScannedSongs && (
          <button className="backup-button" type="button" onClick={() => setIsBackupWizardOpen(true)} disabled={!canCategorize || isScanBusy || isCleanBusy || isRestoreBusy || isInstrumentAnalyzeBusy || isGameIconBusy}>Backup</button>
        )}
      </div>

      {hasScannedSongs && songIniScanResult && (
        <ScannedSongsTable
          duplicateChecksumGroups={songIniScanResult.duplicate_checksum_groups}
          includedSongPaths={includedSongPaths}
          onCategorizationStateChange={setCategorizationTableState}
          onDraftMetadataChange={setDraftMetadata}
          onCopyFolderPath={copyContentIssuePath}
          onRestoreOriginal={restoreScannedSongOriginal}
          onSaveMetadata={saveScannedSongMetadata}
          onSetSongIncluded={setSongIncluded}
          onSetSongsIncluded={setSongsIncluded}
          restoringSongPaths={restoringScannedSongPaths}
          savingSongPaths={savingScannedSongPaths}
          importedMetadataByPath={importedMetadataByPath}
          onImportedMetadataApplied={() => setImportedMetadataByPath({})}
          songs={songIniScanResult?.songs ?? []}
        />
      )}

      <div className="app-controls">
        <button
          className="experimental-warning-button"
          type="button"
          onClick={() => setIsExperimentalWarningOpen(true)}
        >
          Experimental software, use at your own risk!!!
        </button>
        <button
          className="about-button"
          type="button"
          onClick={() => setIsAboutOpen(true)}
        >
          About
        </button>
        <button
          className="settings-button"
          type="button"
          onClick={() => setIsSettingsOpen(true)}
          aria-label="Open settings"
        >
          Settings
        </button>
      </div>

      {isSettingsOpen && (
        <SettingsDialog
          canClose={canCloseSettings}
          onAcceptDisclaimer={acceptDisclaimer}
          onChangeGameLogosFolder={chooseGameLogosFolder}
          onChangeFolder={chooseModsFolder}
          onClose={() => {
            if (canCloseSettings) {
              setIsSettingsOpen(false);
            }
          }}
          onCheckForUpdatesOnStartupChange={setCheckForUpdatesOnStartup}
          onKeepOriginalSongIniChange={setKeepOriginalSongIni}
          settings={settings}
          settingsError={settingsError}
          settingsStatus={settingsStatus}
        />
      )}

      {isExperimentalWarningOpen && (
        <ExperimentalWarningDialog
          onClose={() => setIsExperimentalWarningOpen(false)}
        />
      )}

      {isAboutOpen && (
        <AboutDialog
          onClose={() => setIsAboutOpen(false)}
          onOpenThirdPartyLicenses={() => setIsThirdPartyLicensesOpen(true)}
        />
      )}

      {isThirdPartyLicensesOpen && (
        <ThirdPartyLicensesDialog
          onClose={() => setIsThirdPartyLicensesOpen(false)}
        />
      )}

      {shouldShowUpdatePopup && (
        <UpdateDialog
          downloadProgress={downloadProgress}
          errorMessage={errorMessage}
          onDisableStartupChecks={disableStartupUpdateChecks}
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
          onSetDuplicateSongIncluded={setDuplicateSongIncluded}
          onSelectSongIni={clearSongIniValidationError}
          onUndoSongIni={undoSongIni}
          onValidateSongIni={validateSongIni}
          onVerifyContentIssueSongs={verifyContentIssueSongs}
          deletedSongIniConflictPaths={deletedSongIniConflictPaths}
          disabledSongIniPaths={disabledSongIniPaths}
          includedSongPaths={includedSongPaths}
          isConflictsOnly={isScanWizardConflictsOnly}
          originalFaultySongIniFiles={originalFaultySongIniFiles}
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

      {isGameIconWizardOpen && (
        <FixGameIconsWizard
          error={gameIconCategoryError}
          fixPreview={gameIconSongFixPreview}
          onApplyFixes={applyGameIconSongFixRows}
          onClose={closeGameIconCategories}
          onFix={fixGameIconCategoryFolders}
          onPreviewFixes={previewGameIconSongFixRows}
          result={gameIconCategoryResult}
          status={gameIconCategoryStatus}
        />
      )}

      {isOfficialCategoryWizardOpen && (
        <FixModsFolderWizard
          error={officialCategoryError}
          folderError={folderSanitizeError}
          folderResult={folderSanitizeResult}
          folderStatus={folderSanitizeStatus}
          onClose={closeOfficialCategories}
          onCopyPath={copyContentIssuePath}
          onDisableAll={disableAllOfficialCategories}
          onDisable={disableOfficialCategoryRow}
          onEnable={enableOfficialCategoryRow}
          onScanFolders={refreshModsFolderNames}
          onSanitizeFolders={sanitizeModsFolders}
          onVerifyAll={refreshOfficialCategories}
          result={officialCategoryResult}
          status={officialCategoryStatus}
        />
      )}

      {isCategorizeWizardOpen && (
        <CategorizeWizard
          error={categorizeSongsError}
          hasActiveFilters={categorizationTableState.hasActiveFilters}
          hasUnsavedSongChanges={
            categorizationTableState.hasUnsavedSongChanges
          }
          includedSongCount={includedSongPaths.length}
          includedSongPaths={includedSongPaths}
          modsDir={settings?.mods_dir ?? null}
          onApply={applyCategorization}
          onClose={() => setIsCategorizeWizardOpen(false)}
          orderedSongPaths={categorizationTableState.orderedSongPaths}
          songs={songIniScanResult?.songs ?? []}
          sortKey={categorizationTableState.sortKey}
          status={categorizeSongsStatus}
        />
      )}

      {isBackupWizardOpen && songIniScanResult && (
        <BackupWizard
          songs={songIniScanResult.songs}
          includedSongPaths={includedSongPaths}
          draftMetadata={draftMetadata}
          onClose={() => setIsBackupWizardOpen(false)}
          onApply={(metadata, included) => {
            setImportedMetadataByPath(metadata);
            Object.entries(included).forEach(([path, isIncluded]) => setSongIncluded(path, isIncluded));
          }}
          onExported={(path) => setBackupToast({
            message: `Song backup exported to ${path}`,
            tone: "success",
          })}
        />
      )}

      {activeToast && (
        <ScanToast onDismiss={dismissActiveToast} toast={activeToast} />
      )}
    </main>
  );
}

export default App;
