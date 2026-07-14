import { useEffect, useMemo, useState } from "react";

import type { ScanModsStatus } from "../hooks/useModsScanner";
import type { SongIniScanResult, SongScanProgress } from "../types/scanMods";

type ScanWizardProps = {
  onCancel: () => void;
  onConfirm: () => void;
  onCopyContentIssuePath: (absolutePath: string) => void;
  onDeleteSongIniConflict: (relativePath: string) => void;
  onDisableSongIni: (relativePath: string) => void;
  onEnableSongIni: (relativePath: string) => void;
  onSetDuplicateSongIncluded: (
    relativePath: string,
    isIncluded: boolean,
  ) => Promise<string | null>;
  onSelectSongIni: () => void;
  onUndoSongIni: (relativePath: string) => void | Promise<void>;
  onValidateSongIni: (relativePath: string, contents: string) => void;
  onVerifyContentIssueSong: (relativePath: string) => void;
  deletedSongIniConflictPaths: string[];
  disabledSongIniPaths: string[];
  includedSongPaths: string[];
  isConflictsOnly: boolean;
  originalFaultySongIniFiles: Record<
    string,
    { contents: string; error: string; relative_path: string }
  >;
  repairedSongIniPaths: string[];
  scanError: string;
  scanStatus: ScanModsStatus;
  songIniConflictError: string;
  songIniScanResult: SongIniScanResult | null;
  songScanProgress: SongScanProgress | null;
  songIniValidationError: string;
  verifiedContentIssueSongPaths: string[];
  verifyingContentIssueSongPath: string;
};

function scanProgressLabel(progress: SongScanProgress) {
  if (progress.phase === "findingSongs") {
    return "Finding song INI files...";
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
  onEnableSongIni,
  onSetDuplicateSongIncluded,
  onSelectSongIni,
  onUndoSongIni,
  onValidateSongIni,
  onVerifyContentIssueSong,
  deletedSongIniConflictPaths,
  disabledSongIniPaths,
  includedSongPaths,
  isConflictsOnly,
  originalFaultySongIniFiles,
  repairedSongIniPaths,
  scanError,
  scanStatus,
  songIniConflictError,
  songIniScanResult,
  songScanProgress,
  songIniValidationError,
  verifiedContentIssueSongPaths,
  verifyingContentIssueSongPath,
}: ScanWizardProps) {
  const [activeStep, setActiveStep] = useState<1 | 2 | 3>(() =>
    isConflictsOnly ? 2 : 1,
  );
  const [selectedSongIniPath, setSelectedSongIniPath] = useState("");
  const [editedSongIniContents, setEditedSongIniContents] = useState<
    Record<string, string>
  >({});
  const [touchedDuplicateChecksums, setTouchedDuplicateChecksums] = useState<
    string[]
  >([]);
  const [duplicateGroupSnapshots, setDuplicateGroupSnapshots] = useState<
    Record<string, string[]>
  >({});
  const isScanningSongs = scanStatus === "scanningSongs";
  const isValidatingSong = scanStatus === "validatingSong";
  const isUndoingSong = scanStatus === "undoingSong";
  const isVerifyingSong = scanStatus === "verifyingSong";
  const isDisablingSong = scanStatus === "disablingSong";
  const isEnablingSong = scanStatus === "enablingSong";
  const isExcludingSong = scanStatus === "excludingSong";
  const isDeletingSongConflict = scanStatus === "deletingSongConflict";
  const isBusy =
    isScanningSongs ||
    isValidatingSong ||
    isUndoingSong ||
    isVerifyingSong ||
    isDisablingSong ||
    isEnablingSong ||
    isExcludingSong ||
    isDeletingSongConflict;
  const isScanWorkflow =
    scanStatus === "scanningSongs" ||
    scanStatus === "readySongRepair" ||
    scanStatus === "validatingSong" ||
    scanStatus === "undoingSong" ||
    scanStatus === "verifyingSong" ||
    scanStatus === "disablingSong" ||
    scanStatus === "enablingSong" ||
    scanStatus === "excludingSong" ||
    scanStatus === "deletingSongConflict";
  const repairedPathSet = useMemo(
    () => new Set(repairedSongIniPaths),
    [repairedSongIniPaths],
  );
  const disabledPathSet = useMemo(
    () => new Set(disabledSongIniPaths),
    [disabledSongIniPaths],
  );
  const includedPathSet = useMemo(
    () => new Set(includedSongPaths),
    [includedSongPaths],
  );
  const deletedPathSet = useMemo(
    () => new Set(deletedSongIniConflictPaths),
    [deletedSongIniConflictPaths],
  );
  const faultySongIniFiles = songIniScanResult?.faulty_files ?? [];
  const unresolvedSongIniCount = faultySongIniFiles.filter(
    (file) =>
      !repairedPathSet.has(file.relative_path) &&
      !disabledPathSet.has(file.relative_path),
  ).length;
  const duplicateChecksumGroups =
    songIniScanResult?.duplicate_checksum_groups ?? [];
  const displayedDuplicateChecksumGroups = duplicateChecksumGroups
    .map((group) => ({
      ...group,
      relative_paths: group.relative_paths.filter(
        (path) => !deletedPathSet.has(path),
      ),
    }))
    .filter((group) => group.relative_paths.length > 0);
  const songIniFolderConflicts =
    songIniScanResult?.song_ini_folder_conflicts ?? [];
  const contentFileIssues = songIniScanResult?.content_file_issues ?? [];
  const songByPath = useMemo(
    () =>
      new Map(
        (songIniScanResult?.songs ?? []).map((song) => [
          song.relative_path,
          song,
        ]),
      ),
    [songIniScanResult],
  );
  const unresolvedDuplicateChecksumGroups = duplicateChecksumGroups.filter(
    (group) =>
      group.relative_paths.filter(
        (path) =>
          includedPathSet.has(path) &&
          !disabledPathSet.has(path) &&
          !deletedPathSet.has(path),
      ).length > 1,
  );
  const touchedDuplicateChecksumSet = new Set(touchedDuplicateChecksums);
  const duplicateChecksumGroupsWithSnapshots = [
    ...displayedDuplicateChecksumGroups.map((group) => ({
      ...group,
      relative_paths: (
        duplicateGroupSnapshots[group.checksum] ?? group.relative_paths
      ).filter((path) => !deletedPathSet.has(path)),
    })),
    ...touchedDuplicateChecksums
      .filter(
        (checksum) =>
          !displayedDuplicateChecksumGroups.some(
            (group) => group.checksum === checksum,
          ),
      )
      .map((checksum) => ({
        checksum,
        relative_paths: (duplicateGroupSnapshots[checksum] ?? []).filter(
          (path) => !deletedPathSet.has(path),
        ),
      })),
  ];
  const visibleDuplicateChecksumGroups = duplicateChecksumGroupsWithSnapshots
    .filter(
      (group) =>
        group.relative_paths.length > 0 &&
        (group.relative_paths.filter(
        (path) =>
          includedPathSet.has(path) &&
          !disabledPathSet.has(path) &&
          !deletedPathSet.has(path),
        ).length > 1 || touchedDuplicateChecksumSet.has(group.checksum)),
    );
  const unresolvedSongIniFolderConflicts = songIniFolderConflicts
    .map((conflict) => ({
      ...conflict,
      file_paths: conflict.file_paths.filter(
        (path) => !deletedPathSet.has(path),
      ),
    }))
    .filter((conflict) => conflict.file_paths.length > 1);
  const hasUnresolvedFileConflicts =
    unresolvedSongIniFolderConflicts.length > 0;
  const shouldShowDuplicateChecksumGroups =
    !hasUnresolvedFileConflicts && visibleDuplicateChecksumGroups.length > 0;
  const hasConflictStep = songIniScanResult !== null;
  const unresolvedContentFileIssues = contentFileIssues.filter(
    (issue) =>
      !disabledPathSet.has(issue.song_ini_relative_path) &&
      !deletedPathSet.has(issue.song_ini_relative_path),
  );
  const unresolvedMissingContentFileIssues = unresolvedContentFileIssues.filter(
    (issue) => issue.message === "Missing required file.",
  );
  const unresolvedUnexpectedContentFileIssues =
    unresolvedContentFileIssues.filter(
      (issue) => issue.message === "Unexpected file or folder.",
    );
  const hasContentStep = songIniScanResult !== null;
  const contentIssuesBySong = useMemo(() => {
    const groups = new Map<string, typeof unresolvedContentFileIssues>();

    for (const issue of unresolvedContentFileIssues) {
      const group = groups.get(issue.song_ini_relative_path) ?? [];

      group.push(issue);
      groups.set(issue.song_ini_relative_path, group);
    }

    const rows = Array.from(groups.entries()).map(([relativePath, issues]) => ({
      relativePath,
      issues,
      folderAbsolutePath:
        absoluteParentPath(issues[0].song_ini_absolute_path) ||
        songByPath.get(relativePath)?.folder_absolute_path ||
        "",
    }));

    for (const relativePath of verifiedContentIssueSongPaths) {
      if (
        groups.has(relativePath) ||
        disabledPathSet.has(relativePath) ||
        deletedPathSet.has(relativePath)
      ) {
        continue;
      }

      const song = songByPath.get(relativePath);

      if (song) {
        rows.push({
          relativePath,
          issues: [],
          folderAbsolutePath: song.folder_absolute_path,
        });
      }
    }

    return rows;
  }, [
    deletedPathSet,
    disabledPathSet,
    songByPath,
    unresolvedContentFileIssues,
    verifiedContentIssueSongPaths,
  ]);
  const unresolvedConflictCount =
    unresolvedDuplicateChecksumGroups.length + unresolvedSongIniFolderConflicts.length;
  const unresolvedContentIssueCount = unresolvedContentFileIssues.length;
  const unresolvedMissingContentIssueCount =
    unresolvedMissingContentFileIssues.length;
  const selectedSongIniFile = faultySongIniFiles.find(
    (file) => file.relative_path === selectedSongIniPath,
  );
  const isSelectedSongIniDisabled = disabledPathSet.has(selectedSongIniPath);
  const selectedOriginalSongIniFile =
    originalFaultySongIniFiles[selectedSongIniPath];
  const selectedSongIniContents =
    selectedSongIniPath in editedSongIniContents
      ? editedSongIniContents[selectedSongIniPath]
      : (selectedSongIniFile?.contents ?? "");
  const hasSelectedSongIniDraftChanges =
    Boolean(selectedOriginalSongIniFile) &&
    selectedSongIniContents !== selectedOriginalSongIniFile?.contents;
  const canUndoSelectedSongIni =
    Boolean(selectedOriginalSongIniFile) &&
    (repairedPathSet.has(selectedSongIniPath) ||
      hasSelectedSongIniDraftChanges) &&
    !isSelectedSongIniDisabled;
  const canFinishSongIniStep =
    activeStep === 1 && isScanWorkflow && unresolvedSongIniCount === 0;
  const canFinishConflictStep = activeStep === 2 && unresolvedConflictCount === 0;
  const canFinishContentStep =
    activeStep === 3 && unresolvedMissingContentIssueCount === 0;
  const title =
    activeStep === 1
      ? "Check song INI format"
      : activeStep === 2
        ? "Resolve song conflicts"
        : "Check content files";
  const stepLabel = `Step ${activeStep} of 3`;
  const scanProgressText = songScanProgress
    ? scanProgressLabel(songScanProgress)
    : "Checking song INI format...";
  const hasNextStep =
    !isConflictsOnly &&
    ((activeStep === 1 && (hasConflictStep || hasContentStep)) ||
      (activeStep === 2 && hasContentStep));

  function isSongIniStep() {
    return activeStep === 1 && isScanWorkflow;
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
    if (isConflictsOnly) {
      return;
    }

    if (activeStep === 3 && hasConflictStep) {
      setActiveStep(2);
      return;
    }

    setActiveStep(1);
  }

  function continueOrFinish() {
    if (isConflictsOnly) {
      onConfirm();
      return;
    }

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
      setTouchedDuplicateChecksums([]);
      setDuplicateGroupSnapshots({});
      return;
    }

    if (isConflictsOnly) {
      setActiveStep(2);
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
  }, [isConflictsOnly, songIniScanResult]);

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
        {hasUnresolvedFileConflicts
          ? `${unresolvedSongIniFolderConflicts.length} same-folder file conflict${
              unresolvedSongIniFolderConflicts.length === 1 ? "" : "s"
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
          {visibleDuplicateChecksumGroups.map((group) => {
            const activePaths = group.relative_paths.filter(
              (path) =>
                includedPathSet.has(path) &&
                !disabledPathSet.has(path) &&
                !deletedPathSet.has(path),
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
                    const isIncluded = includedPathSet.has(path);
                    const isUnavailable = isDisabled || isDeleted;

                    return (
                      <li key={path}>
                        <span>{path}</span>
                        <button
                          className="secondary-btn"
                          type="button"
                          onClick={async () => {
                            setTouchedDuplicateChecksums((currentChecksums) =>
                              currentChecksums.includes(group.checksum)
                                ? currentChecksums
                                : [...currentChecksums, group.checksum],
                            );
                            const nextPath = await onSetDuplicateSongIncluded(
                              path,
                              !isIncluded,
                            );

                            if (!nextPath) {
                              return;
                            }

                            setDuplicateGroupSnapshots((currentSnapshots) => {
                              const currentPaths =
                                currentSnapshots[group.checksum] ??
                                group.relative_paths;

                              return {
                                ...currentSnapshots,
                                [group.checksum]: currentPaths.map(
                                  (currentPath) =>
                                    currentPath === path ? nextPath : currentPath,
                                ),
                              };
                            });
                          }}
                          disabled={isBusy || isUnavailable}
                        >
                          {isUnavailable
                            ? "Unavailable"
                            : isIncluded
                              ? "Exclude"
                              : "Include"}
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

      {hasUnresolvedFileConflicts && (
        <div className="song-conflict-section">
          <h3>Song INI files</h3>
          {unresolvedSongIniFolderConflicts.map((conflict) => (
            <div className="song-conflict-group" key={conflict.folder_path}>
              <div className="song-conflict-group-header">
                <span>{conflict.folder_path}</span>
                <span>Conflict</span>
              </div>
              <ul className="song-conflict-list">
                {conflict.file_paths.map((path) => (
                  <li key={path}>
                    <span>{fileName(path)}</span>
                    <button
                      className="secondary-btn"
                      type="button"
                      onClick={() => onDeleteSongIniConflict(path)}
                      disabled={isBusy}
                    >
                      Delete
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          ))}
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
        {unresolvedContentIssueCount === 1 ? "" : "s"} found. {" "}
        {unresolvedMissingContentIssueCount} missing required file
        {unresolvedMissingContentIssueCount === 1 ? "" : "s"} must be fixed
        before finishing.
      </p>

      {unresolvedUnexpectedContentFileIssues.length > 0 && (
        <p className="scan-wizard-muted">
          Unexpected files and folders are listed for review but do not need to
          be removed.
        </p>
      )}

      {songIniConflictError && (
        <p className="scan-wizard-error">{songIniConflictError}</p>
      )}

      {contentIssuesBySong.map(({
        relativePath,
        issues,
        folderAbsolutePath,
      }) => {
        const firstIssue = issues[0];
        const isDisabled = disabledPathSet.has(relativePath);
        const isDeleted = deletedPathSet.has(relativePath);
        const isVerifiedFine = issues.length === 0;
        const isVerifyingThisSong =
          isVerifyingSong && verifyingContentIssueSongPath === relativePath;

        return (
          <div className="song-conflict-group" key={relativePath}>
            <div className="song-conflict-group-header">
              <span>
                {relativePath}
                {firstIssue ? ` (${firstIssue.checksum})` : ""}
              </span>
              <div className="song-conflict-actions">
                <button
                  className="secondary-btn"
                  type="button"
                  onClick={() => onCopyContentIssuePath(folderAbsolutePath)}
                  disabled={isBusy || !folderAbsolutePath}
                >
                  Copy path
                </button>
                <button
                  className="secondary-btn"
                  type="button"
                  onClick={() => onVerifyContentIssueSong(relativePath)}
                  disabled={isBusy || isDisabled || isDeleted}
                >
                  {isVerifyingThisSong ? "Verifying..." : "Verify"}
                </button>
                <button
                  className="secondary-btn"
                  type="button"
                  onClick={() => onDisableSongIni(relativePath)}
                  disabled={isBusy || isDisabled || isDeleted}
                >
                  {isDisabled ? "Disabled" : "Disable"}
                </button>
              </div>
            </div>
            <ul className="song-conflict-list">
              {isVerifiedFine ? (
                <li className="song-content-issue-row">
                  <span>
                    <strong>Fine</strong>
                    <small>No content file issues found.</small>
                  </span>
                </li>
              ) : (
                issues.map((issue) => (
                  <li
                    className={`song-content-issue-row${
                      issue.message === "Missing required file."
                        ? " song-content-issue-row-error"
                        : ""
                    }`}
                    key={`${issue.absolute_path}-${issue.message}`}
                  >
                    <span>
                      <strong>{issue.message}</strong>
                      <small>{contentIssueDisplayPath(issue)}</small>
                    </span>
                  </li>
                ))
              )}
            </ul>
          </div>
        );
      })}

      {hasContentStep &&
        unresolvedMissingContentIssueCount === 0 &&
        contentIssuesBySong.length === 0 && (
        <p className="scan-wizard-muted">
          No missing required content files were found.
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
            <p className="scan-wizard-step">
              {isConflictsOnly ? "Duplicate resolver" : stepLabel}
            </p>
            <h2 id="scan-wizard-title">{title}</h2>
          </div>
          {!isConflictsOnly && (
            <button
              className="close-btn"
              type="button"
              onClick={onCancel}
              aria-label="Cancel scan"
              disabled={isBusy}
            >
              &times;
            </button>
          )}
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
                {songIniScanResult.songs_found} song INI files passed format
                checks.
              </p>

              {songIniScanResult.errors.map((error, index) => (
                <p className="scan-wizard-error" key={`${index}-${error}`}>
                  {error}
                </p>
              ))}

              {faultySongIniFiles.length ? (
                <div className="song-ini-repair">
                  <ul className="song-ini-file-list" aria-label="Faulty song INI files">
                    {faultySongIniFiles.map((file) => {
                      const isSelected =
                        selectedSongIniPath === file.relative_path;
                      const isRepaired = repairedPathSet.has(file.relative_path);
                      const isDisabled = disabledPathSet.has(file.relative_path);
                      const statusLabel = isDisabled
                        ? "Disabled"
                        : isRepaired
                          ? "Fine"
                          : "Error";

                      return (
                        <li key={file.relative_path}>
                          <button
                            className={[
                              "song-ini-file-button",
                              isSelected ? "selected" : "",
                              isRepaired || isDisabled ? "repaired" : "",
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
                            <span>{statusLabel}</span>
                          </button>
                        </li>
                      );
                    })}
                  </ul>

                  <div className="song-ini-editor">
                    {selectedSongIniFile ? (
                      <>
                        {isSelectedSongIniDisabled ? (
                          <p className="scan-wizard-muted">
                            This song INI has been disabled.
                          </p>
                        ) : selectedSongIniFile.error ? (
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
                        {songIniConflictError && (
                          <p className="scan-wizard-error">
                            {songIniConflictError}
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
                        <div className="song-ini-editor-actions">
                          <button
                            className="secondary-btn"
                            type="button"
                            onClick={() =>
                              isSelectedSongIniDisabled
                                ? onEnableSongIni(selectedSongIniPath)
                                : onDisableSongIni(selectedSongIniPath)
                            }
                            disabled={isBusy || !selectedSongIniPath}
                          >
                            {isDisablingSong
                              ? "Disabling..."
                              : isEnablingSong
                                ? "Enabling..."
                              : isSelectedSongIniDisabled
                                ? "Enable"
                                : "Disable"}
                          </button>
                          <button
                            className="secondary-btn"
                            type="button"
                            onClick={async () => {
                              const originalFile =
                                originalFaultySongIniFiles[
                                  selectedSongIniPath
                                ];

                              if (originalFile) {
                                setEditedSongIniContents(
                                  (currentContents) => ({
                                    ...currentContents,
                                    [selectedSongIniPath]:
                                      originalFile.contents,
                                  }),
                                );
                              }

                              await onUndoSongIni(selectedSongIniPath);
                            }}
                            disabled={
                              isBusy ||
                              !selectedSongIniPath ||
                              !canUndoSelectedSongIni
                            }
                          >
                            {isUndoingSong ? "Undoing..." : "Undo"}
                          </button>
                          <button
                            className="primary-btn"
                            type="button"
                            onClick={() =>
                              onValidateSongIni(
                                selectedSongIniPath,
                                selectedSongIniContents,
                              )
                            }
                            disabled={
                              isBusy ||
                              !selectedSongIniPath ||
                              isSelectedSongIniDisabled
                            }
                          >
                            {isValidatingSong ? "Validating..." : "Validate"}
                          </button>
                        </div>
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
          {activeStep !== 1 && !isConflictsOnly && (
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
