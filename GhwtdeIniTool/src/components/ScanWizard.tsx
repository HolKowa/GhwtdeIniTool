import { useEffect, useMemo, useState } from "react";

import type { ScanModsStatus } from "../hooks/useModsScanner";
import type { SongIniScanResult, SongScanProgress } from "../types/scanMods";

type ScanWizardProps = {
  onCancel: () => void;
  onConfirm: () => void;
  onCopyContentIssuePath: (absolutePath: string) => void;
  onDeleteSongIniConflict: (relativePath: string) => void;
  onDisableSongIni: (relativePath: string) => void;
  onSelectSongIni: () => void;
  onValidateSongIni: (relativePath: string, contents: string) => void;
  deletedSongIniConflictPaths: string[];
  disabledSongIniPaths: string[];
  repairedSongIniPaths: string[];
  scanError: string;
  scanStatus: ScanModsStatus;
  songIniConflictError: string;
  songIniScanResult: SongIniScanResult | null;
  songScanProgress: SongScanProgress | null;
  songIniValidationError: string;
};

function scanProgressLabel(progress: SongScanProgress) {
  if (progress.phase === "findingSongs") {
    return "Finding song.ini files...";
  }

  if (progress.phase === "readingSongs") {
    return `Reading song ${progress.current} of ${progress.total}`;
  }

  if (progress.phase === "checkingContent") {
    return `Checking content ${progress.current} of ${progress.total}`;
  }

  return "Finishing scan...";
}

export function ScanWizard({
  onCancel,
  onConfirm,
  onCopyContentIssuePath,
  onDeleteSongIniConflict,
  onDisableSongIni,
  onSelectSongIni,
  onValidateSongIni,
  deletedSongIniConflictPaths,
  disabledSongIniPaths,
  repairedSongIniPaths,
  scanError,
  scanStatus,
  songIniConflictError,
  songIniScanResult,
  songScanProgress,
  songIniValidationError,
}: ScanWizardProps) {
  const [activeStep, setActiveStep] = useState<1 | 2 | 3>(1);
  const [selectedSongIniPath, setSelectedSongIniPath] = useState("");
  const [editedSongIniContents, setEditedSongIniContents] = useState<
    Record<string, string>
  >({});
  const isScanningSongs = scanStatus === "scanningSongs";
  const isValidatingSong = scanStatus === "validatingSong";
  const isDisablingSong = scanStatus === "disablingSong";
  const isDeletingSongConflict = scanStatus === "deletingSongConflict";
  const isBusy =
    isScanningSongs ||
    isValidatingSong ||
    isDisablingSong ||
    isDeletingSongConflict;
  const isScanWorkflow =
    scanStatus === "scanningSongs" ||
    scanStatus === "readySongRepair" ||
    scanStatus === "validatingSong" ||
    scanStatus === "disablingSong" ||
    scanStatus === "deletingSongConflict";
  const repairedPathSet = useMemo(
    () => new Set(repairedSongIniPaths),
    [repairedSongIniPaths],
  );
  const disabledPathSet = useMemo(
    () => new Set(disabledSongIniPaths),
    [disabledSongIniPaths],
  );
  const deletedPathSet = useMemo(
    () => new Set(deletedSongIniConflictPaths),
    [deletedSongIniConflictPaths],
  );
  const faultySongIniFiles = songIniScanResult?.faulty_files ?? [];
  const unresolvedSongIniCount = faultySongIniFiles.filter(
    (file) => !repairedPathSet.has(file.relative_path),
  ).length;
  const duplicateChecksumGroups =
    songIniScanResult?.duplicate_checksum_groups ?? [];
  const disabledSongConflicts =
    songIniScanResult?.disabled_song_conflicts ?? [];
  const contentFileIssues = songIniScanResult?.content_file_issues ?? [];
  const unresolvedDuplicateChecksumGroups = duplicateChecksumGroups.filter(
    (group) =>
      group.relative_paths.filter(
        (path) => !disabledPathSet.has(path) && !deletedPathSet.has(path),
      ).length > 1,
  );
  const unresolvedDisabledSongConflicts = disabledSongConflicts.filter(
    (conflict) =>
      !deletedPathSet.has(conflict.active_path) &&
      !deletedPathSet.has(conflict.disabled_path),
  );
  const hasUnresolvedDisabledSongConflicts =
    unresolvedDisabledSongConflicts.length > 0;
  const shouldShowDuplicateChecksumGroups =
    !hasUnresolvedDisabledSongConflicts && duplicateChecksumGroups.length > 0;
  const hasConflictStep = songIniScanResult !== null;
  const unresolvedContentFileIssues = contentFileIssues.filter(
    (issue) =>
      !disabledPathSet.has(issue.song_ini_relative_path) &&
      !deletedPathSet.has(issue.song_ini_relative_path),
  );
  const hasContentStep = songIniScanResult !== null;
  const contentIssuesBySong = useMemo(() => {
    const groups = new Map<string, typeof unresolvedContentFileIssues>();

    for (const issue of unresolvedContentFileIssues) {
      const group = groups.get(issue.song_ini_relative_path) ?? [];

      group.push(issue);
      groups.set(issue.song_ini_relative_path, group);
    }

    return Array.from(groups.entries()).map(([relativePath, issues]) => ({
      relativePath,
      issues,
    }));
  }, [unresolvedContentFileIssues]);
  const unresolvedConflictCount =
    unresolvedDuplicateChecksumGroups.length +
    unresolvedDisabledSongConflicts.length;
  const unresolvedContentIssueCount = unresolvedContentFileIssues.length;
  const selectedSongIniFile = faultySongIniFiles.find(
    (file) => file.relative_path === selectedSongIniPath,
  );
  const selectedSongIniContents =
    selectedSongIniPath in editedSongIniContents
      ? editedSongIniContents[selectedSongIniPath]
      : (selectedSongIniFile?.contents ?? "");
  const canFinishSongIniStep =
    activeStep === 1 && isScanWorkflow && unresolvedSongIniCount === 0;
  const canFinishConflictStep = activeStep === 2 && unresolvedConflictCount === 0;
  const canFinishContentStep =
    activeStep === 3 && unresolvedContentIssueCount === 0;
  const title =
    activeStep === 1
      ? "Check song.ini format"
      : activeStep === 2
        ? "Resolve song conflicts"
        : "Check content files";
  const stepLabel = `Step ${activeStep} of 3`;
  const scanProgressText = songScanProgress
    ? scanProgressLabel(songScanProgress)
    : "Checking song.ini format...";
  const hasNextStep =
    (activeStep === 1 && (hasConflictStep || hasContentStep)) ||
    (activeStep === 2 && hasContentStep);

  function isSongIniStep() {
    return activeStep === 1 && isScanWorkflow;
  }

  function folderPath(relativePath: string) {
    const lastSeparatorIndex = relativePath.lastIndexOf("/");

    return lastSeparatorIndex === -1
      ? "."
      : relativePath.slice(0, lastSeparatorIndex);
  }

  function fileName(relativePath: string) {
    const lastSeparatorIndex = relativePath.lastIndexOf("/");

    return lastSeparatorIndex === -1
      ? relativePath
      : relativePath.slice(lastSeparatorIndex + 1);
  }

  function contentIssueDisplayPath(issue: {
    absolute_path: string;
    song_ini_absolute_path: string;
  }) {
    const songFolderPath = absoluteParentPath(issue.song_ini_absolute_path);
    const pathFromSongFolder = issue.absolute_path.slice(songFolderPath.length);

    return pathFromSongFolder.replace(/^[/\\]/, "");
  }

  function absoluteParentPath(absolutePath: string) {
    return absolutePath.replace(/[/\\][^/\\]*$/, "");
  }

  function goBack() {
    if (activeStep === 3 && hasConflictStep) {
      setActiveStep(2);
      return;
    }

    setActiveStep(1);
  }

  function continueOrFinish() {
    if (activeStep === 1 && hasConflictStep) {
      setActiveStep(2);
      return;
    }

    if (activeStep === 1 && hasContentStep) {
      setActiveStep(3);
      return;
    }

    if (activeStep === 2 && hasContentStep) {
      setActiveStep(3);
      return;
    }

    onConfirm();
  }

  useEffect(() => {
    if (!songIniScanResult) {
      setActiveStep(1);
      setSelectedSongIniPath("");
      setEditedSongIniContents({});
      return;
    }

    setEditedSongIniContents((currentContents) => {
      const nextContents = { ...currentContents };

      for (const file of songIniScanResult.faulty_files) {
        if (!file.error || !(file.relative_path in nextContents)) {
          nextContents[file.relative_path] = file.contents;
        }
      }

      return nextContents;
    });
  }, [songIniScanResult]);

  useEffect(() => {
    if (!faultySongIniFiles.length) {
      setSelectedSongIniPath("");
      return;
    }

    const selectedPathExists = faultySongIniFiles.some(
      (file) => file.relative_path === selectedSongIniPath,
    );

    if (selectedPathExists) {
      return;
    }

    const firstUnresolvedFile =
      faultySongIniFiles.find(
        (file) => !repairedPathSet.has(file.relative_path),
      ) ?? faultySongIniFiles[0];

    setSelectedSongIniPath(firstUnresolvedFile.relative_path);
  }, [faultySongIniFiles, repairedPathSet, selectedSongIniPath]);

  const renderConflictStep = () => (
    <>
      <p className="scan-wizard-summary">
        {hasUnresolvedDisabledSongConflicts
          ? `${unresolvedDisabledSongConflicts.length} active/disabled file conflict${
              unresolvedDisabledSongConflicts.length === 1 ? "" : "s"
            } remaining. Resolve these before duplicate checksums.`
          : `${unresolvedConflictCount} conflict group${
              unresolvedConflictCount === 1 ? "" : "s"
            } remaining.`}
      </p>

      {songIniConflictError && (
        <p className="scan-wizard-error">{songIniConflictError}</p>
      )}

      {shouldShowDuplicateChecksumGroups && (
        <div className="song-conflict-section">
          <h3>Duplicate checksums</h3>
          {duplicateChecksumGroups.map((group) => {
            const activePaths = group.relative_paths.filter(
              (path) => !disabledPathSet.has(path) && !deletedPathSet.has(path),
            );
            const isResolved = activePaths.length <= 1;

            return (
              <div className="song-conflict-group" key={group.checksum}>
                <div className="song-conflict-group-header">
                  <span>{group.checksum}</span>
                  <span className={isResolved ? "resolved" : ""}>
                    {isResolved ? "Resolved" : "Duplicate"}
                  </span>
                </div>
                <ul className="song-conflict-list">
                  {group.relative_paths.map((path) => {
                    const isDisabled = disabledPathSet.has(path);
                    const isDeleted = deletedPathSet.has(path);

                    return (
                      <li key={path}>
                        <span>{path}</span>
                        <button
                          className="secondary-btn"
                          type="button"
                          onClick={() => onDisableSongIni(path)}
                          disabled={isBusy || isDisabled || isDeleted}
                        >
                          {isDisabled ? "Disabled" : "Disable"}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </div>
            );
          })}
        </div>
      )}

      {hasUnresolvedDisabledSongConflicts && (
        <div className="song-conflict-section">
          <h3>Active and disabled files</h3>
          {unresolvedDisabledSongConflicts.map((conflict) => {
            const isResolved =
              deletedPathSet.has(conflict.active_path) ||
              deletedPathSet.has(conflict.disabled_path);

            return (
              <div className="song-conflict-group" key={conflict.active_path}>
                <div className="song-conflict-group-header">
                  <span>{folderPath(conflict.active_path)}</span>
                  <span className={isResolved ? "resolved" : ""}>
                    {isResolved ? "Resolved" : "Conflict"}
                  </span>
                </div>
                <ul className="song-conflict-list">
                  <li>
                    <span>{fileName(conflict.active_path)}</span>
                    <button
                      className="secondary-btn"
                      type="button"
                      onClick={() =>
                        onDeleteSongIniConflict(conflict.active_path)
                      }
                      disabled={isBusy || isResolved}
                    >
                      Delete active
                    </button>
                  </li>
                  <li>
                    <span>{fileName(conflict.disabled_path)}</span>
                    <button
                      className="secondary-btn"
                      type="button"
                      onClick={() =>
                        onDeleteSongIniConflict(conflict.disabled_path)
                      }
                      disabled={isBusy || isResolved}
                    >
                      Delete disabled
                    </button>
                  </li>
                </ul>
              </div>
            );
          })}
        </div>
      )}

      {hasConflictStep &&
        unresolvedConflictCount === 0 &&
        !shouldShowDuplicateChecksumGroups && (
          <p className="scan-wizard-muted">
            All song conflicts have been resolved.
          </p>
        )}

      {!hasConflictStep && (
        <p className="scan-wizard-muted">No song conflicts were found.</p>
      )}
    </>
  );

  const renderContentStep = () => (
    <>
      <p className="scan-wizard-summary">
        {unresolvedContentIssueCount} content issue
        {unresolvedContentIssueCount === 1 ? "" : "s"} remaining.
      </p>

      {songIniConflictError && (
        <p className="scan-wizard-error">{songIniConflictError}</p>
      )}

      {contentIssuesBySong.map(({ relativePath, issues }) => {
        const firstIssue = issues[0];
        const isDisabled = disabledPathSet.has(relativePath);
        const isDeleted = deletedPathSet.has(relativePath);

        return (
          <div className="song-conflict-group" key={relativePath}>
            <div className="song-conflict-group-header">
              <span>
                {relativePath} ({firstIssue.checksum})
              </span>
              <div className="song-conflict-actions">
                <button
                  className="secondary-btn"
                  type="button"
                  onClick={() =>
                    onCopyContentIssuePath(
                      absoluteParentPath(firstIssue.song_ini_absolute_path),
                    )
                  }
                  disabled={isBusy}
                >
                  Copy path
                </button>
                <button
                  className="secondary-btn"
                  type="button"
                  onClick={() => onDisableSongIni(relativePath)}
                  disabled={isBusy || isDisabled || isDeleted}
                >
                  {isDisabled ? "Disabled" : "Disable song.ini"}
                </button>
              </div>
            </div>
            <ul className="song-conflict-list">
              {issues.map((issue) => (
                <li
                  className="song-content-issue-row"
                  key={`${issue.absolute_path}-${issue.message}`}
                >
                  <span>
                    <strong>{issue.message}</strong>
                    <small>{contentIssueDisplayPath(issue)}</small>
                  </span>
                </li>
              ))}
            </ul>
          </div>
        );
      })}

      {hasContentStep && unresolvedContentIssueCount === 0 && (
        <p className="scan-wizard-muted">
          All content file issues have been resolved.
        </p>
      )}

      {!hasContentStep && (
        <p className="scan-wizard-muted">No content file issues were found.</p>
      )}
    </>
  );

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="scan-wizard-title"
      >
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">{stepLabel}</p>
            <h2 id="scan-wizard-title">{title}</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onCancel}
            aria-label="Cancel scan"
            disabled={isBusy}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {isScanningSongs && (
            <>
              <p className="scan-wizard-summary">{scanProgressText}</p>
              {songScanProgress?.relative_path && (
                <p className="scan-wizard-muted">
                  {songScanProgress.relative_path}
                </p>
              )}
            </>
          )}

          {scanStatus === "error" && (
            <p className="scan-wizard-error">{scanError}</p>
          )}

          {isSongIniStep() && songIniScanResult && (
            <>
              <p className="scan-wizard-summary">
                {songIniScanResult.songs_parsed} of{" "}
                {songIniScanResult.songs_found} song.ini files passed format
                checks.
              </p>

              {songIniScanResult.errors.map((error, index) => (
                <p className="scan-wizard-error" key={`${index}-${error}`}>
                  {error}
                </p>
              ))}

              {faultySongIniFiles.length ? (
                <div className="song-ini-repair">
                  <ul className="song-ini-file-list" aria-label="Faulty song.ini files">
                    {faultySongIniFiles.map((file) => {
                      const isSelected =
                        selectedSongIniPath === file.relative_path;
                      const isRepaired = repairedPathSet.has(file.relative_path);

                      return (
                        <li key={file.relative_path}>
                          <button
                            className={[
                              "song-ini-file-button",
                              isSelected ? "selected" : "",
                              isRepaired ? "repaired" : "",
                            ]
                              .filter(Boolean)
                              .join(" ")}
                            type="button"
                            onClick={() => {
                              onSelectSongIni();
                              setSelectedSongIniPath(file.relative_path);
                            }}
                          >
                            <span>{file.relative_path}</span>
                            <span>{isRepaired ? "Fine" : "Error"}</span>
                          </button>
                        </li>
                      );
                    })}
                  </ul>

                  <div className="song-ini-editor">
                    {selectedSongIniFile ? (
                      <>
                        {selectedSongIniFile.error ? (
                          <p className="scan-wizard-error">
                            {selectedSongIniFile.error}
                          </p>
                        ) : (
                          <p className="scan-wizard-muted">
                            This song.ini has been validated.
                          </p>
                        )}
                        {songIniValidationError && (
                          <p className="scan-wizard-error">
                            {songIniValidationError}
                          </p>
                        )}
                        <textarea
                          className="song-ini-textarea"
                          spellCheck={false}
                          value={selectedSongIniContents}
                          onChange={(event) =>
                            setEditedSongIniContents((currentContents) => ({
                              ...currentContents,
                              [selectedSongIniPath]: event.target.value,
                            }))
                          }
                        />
                        <button
                          className="primary-btn"
                          type="button"
                          onClick={() =>
                            onValidateSongIni(
                              selectedSongIniPath,
                              selectedSongIniContents,
                            )
                          }
                          disabled={isBusy || !selectedSongIniPath}
                        >
                          {isValidatingSong ? "Validating..." : "Validate"}
                        </button>
                      </>
                    ) : (
                      <p className="scan-wizard-muted">
                        Select a song.ini file to repair.
                      </p>
                    )}
                  </div>
                </div>
              ) : (
                <p className="scan-wizard-muted">
                  No song.ini format issues were found.
                </p>
              )}

              {faultySongIniFiles.length > 0 && unresolvedSongIniCount === 0 && (
                <p className="scan-wizard-summary">
                  All song.ini format issues have been repaired and saved.
                </p>
              )}
            </>
          )}

          {activeStep === 2 && isScanWorkflow && songIniScanResult && (
            renderConflictStep()
          )}

          {activeStep === 3 && isScanWorkflow && songIniScanResult && (
            renderContentStep()
          )}
        </div>

        <div className="scan-wizard-actions">
          {activeStep !== 1 && (
            <button
              className="secondary-btn"
              type="button"
              onClick={goBack}
              disabled={isBusy}
            >
              Back
            </button>
          )}
          <button
            className="primary-btn"
            type="button"
            onClick={continueOrFinish}
            disabled={
              isBusy ||
              scanStatus === "error" ||
              (activeStep === 1 && !canFinishSongIniStep) ||
              (activeStep === 2 && !canFinishConflictStep) ||
              (activeStep === 3 && !canFinishContentStep)
            }
          >
            {isScanningSongs
              ? "Parsing..."
              : hasNextStep
                ? "Continue"
                : "Finish"}
          </button>
        </div>
      </section>
    </div>
  );
}
