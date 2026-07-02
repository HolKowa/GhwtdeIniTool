import { useEffect, useMemo, useState } from "react";

import type { ScanModsStatus } from "../hooks/useModsScanner";
import type { DeleteFilesPreview, SongIniScanResult } from "../types/scanMods";

type ScanWizardProps = {
  deletePreview: DeleteFilesPreview | null;
  onCancel: () => void;
  onConfirm: () => void;
  onSelectSongIni: () => void;
  onValidateSongIni: (relativePath: string, contents: string) => void;
  repairedSongIniPaths: string[];
  scanError: string;
  scanStatus: ScanModsStatus;
  songIniScanResult: SongIniScanResult | null;
  songIniValidationError: string;
};

export function ScanWizard({
  deletePreview,
  onCancel,
  onConfirm,
  onSelectSongIni,
  onValidateSongIni,
  repairedSongIniPaths,
  scanError,
  scanStatus,
  songIniScanResult,
  songIniValidationError,
}: ScanWizardProps) {
  const [selectedSongIniPath, setSelectedSongIniPath] = useState("");
  const [editedSongIniContents, setEditedSongIniContents] = useState<
    Record<string, string>
  >({});
  const isPreviewing = scanStatus === "previewing";
  const isDeleting = scanStatus === "deleting";
  const isScanningSongs = scanStatus === "scanningSongs";
  const isValidatingSong = scanStatus === "validatingSong";
  const isBusy =
    isPreviewing || isDeleting || isScanningSongs || isValidatingSong;
  const isDeleteStep = scanStatus === "readyDelete" || isDeleting;
  const isSongIniStep =
    scanStatus === "scanningSongs" ||
    scanStatus === "readySongRepair" ||
    scanStatus === "validatingSong";
  const hasFilesToDelete = Boolean(deletePreview?.files_to_delete.length);
  const repairedPathSet = useMemo(
    () => new Set(repairedSongIniPaths),
    [repairedSongIniPaths],
  );
  const faultySongIniFiles = songIniScanResult?.faulty_files ?? [];
  const unresolvedSongIniCount = faultySongIniFiles.filter(
    (file) => !repairedPathSet.has(file.relative_path),
  ).length;
  const selectedSongIniFile = faultySongIniFiles.find(
    (file) => file.relative_path === selectedSongIniPath,
  );
  const selectedSongIniContents =
    selectedSongIniPath in editedSongIniContents
      ? editedSongIniContents[selectedSongIniPath]
      : (selectedSongIniFile?.contents ?? "");
  const canFinishSongIniStep = isSongIniStep && unresolvedSongIniCount === 0;

  useEffect(() => {
    if (!songIniScanResult) {
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

  const stepLabel = isSongIniStep ? "Step 2" : "Step 1";
  const title = isSongIniStep
    ? "Validate song.ini files"
    : "Keep only files with pattern";

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
            disabled={isDeleting || isScanningSongs || isValidatingSong}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {isPreviewing && <p className="scan-wizard-muted">Scanning...</p>}
          {isDeleting && <p className="scan-wizard-muted">Deleting files...</p>}
          {isScanningSongs && (
            <p className="scan-wizard-muted">Parsing song.ini files...</p>
          )}

          {scanStatus === "error" && (
            <p className="scan-wizard-error">{scanError}</p>
          )}

          {isDeleteStep && deletePreview && (
            <>
              <p className="scan-wizard-summary">
                {deletePreview.files_to_delete.length} files do not match the
                keep patterns and are ready to delete.
              </p>

              {hasFilesToDelete ? (
                <ul className="scan-file-list">
                  {deletePreview.files_to_delete.map((filePath) => (
                    <li key={filePath}>{filePath}</li>
                  ))}
                </ul>
              ) : (
                <p className="scan-wizard-muted">No files will be deleted.</p>
              )}

              {deletePreview.errors.map((error, index) => (
                <p className="scan-wizard-error" key={`${index}-${error}`}>
                  {error}
                </p>
              ))}
            </>
          )}

          {isSongIniStep && songIniScanResult && (
            <>
              <p className="scan-wizard-summary">
                {songIniScanResult.songs_parsed} of{" "}
                {songIniScanResult.songs_found} song.ini files parsed.
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
                          disabled={isValidatingSong || !selectedSongIniPath}
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
                  No faulty song.ini files were found.
                </p>
              )}

              {faultySongIniFiles.length > 0 && unresolvedSongIniCount === 0 && (
                <p className="scan-wizard-summary">
                  All faulty song.ini files have been validated and saved.
                </p>
              )}
            </>
          )}
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onCancel}
            disabled={isDeleting || isScanningSongs || isValidatingSong}
          >
            Cancel
          </button>
          <button
            className="primary-btn"
            type="button"
            onClick={onConfirm}
            disabled={
              isBusy ||
              scanStatus === "error" ||
              (isSongIniStep && !canFinishSongIniStep)
            }
          >
            {isDeleting
                ? "Deleting..."
                : isScanningSongs
                  ? "Parsing..."
                  : isSongIniStep
                    ? "Finish"
                    : "Confirm"}
          </button>
        </div>
      </section>
    </div>
  );
}
