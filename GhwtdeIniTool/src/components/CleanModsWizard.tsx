import { useState } from "react";

import type { CleanModsStatus } from "../hooks/useModsCleaner";
import type { DeleteFilesPreview } from "../types/scanMods";
import {
  DEFAULT_KEEP_ONLY_FILES_PATTERN,
  validateKeepOnlyFilesPattern,
} from "../utils/keepOnlyFilesPattern";

type CleanModsWizardProps = {
  cleanError: string;
  cleanStatus: CleanModsStatus;
  deletePreview: DeleteFilesPreview | null;
  onCancel: () => void;
  onConfirmDelete: () => void;
  onPreview: (keepOnlyFilesPattern: string) => void;
};

export function CleanModsWizard({
  cleanError,
  cleanStatus,
  deletePreview,
  onCancel,
  onConfirmDelete,
  onPreview,
}: CleanModsWizardProps) {
  const [keepOnlyFilesPattern, setKeepOnlyFilesPattern] = useState(
    DEFAULT_KEEP_ONLY_FILES_PATTERN,
  );
  const keepOnlyFilesPatternError =
    validateKeepOnlyFilesPattern(keepOnlyFilesPattern);
  const isPreviewing = cleanStatus === "previewing";
  const isDeleting = cleanStatus === "deleting";
  const isDeleteStep = cleanStatus === "readyDelete" || isDeleting;
  const hasFilesToDelete = Boolean(deletePreview?.files_to_delete.length);
  const stepLabel = isDeleteStep ? "Step 2" : "Step 1";
  const title = isDeleteStep
    ? "Review files to delete"
    : "Keep only files with pattern";

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="clean-wizard-title"
      >
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">{stepLabel}</p>
            <h2 id="clean-wizard-title">{title}</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onCancel}
            aria-label="Cancel clean"
            disabled={isPreviewing || isDeleting}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {!isDeleteStep && (
            <div className="clean-pattern-form">
              <div className="settings-field-header">
                <label htmlFor="clean-files-pattern">
                  Keep only files with pattern
                </label>
                <button
                  className="primary-btn"
                  type="button"
                  onClick={() =>
                    setKeepOnlyFilesPattern(DEFAULT_KEEP_ONLY_FILES_PATTERN)
                  }
                  disabled={isPreviewing}
                >
                  Keep default
                </button>
              </div>
              <input
                id="clean-files-pattern"
                className={`settings-input${
                  keepOnlyFilesPatternError ? " settings-input-error" : ""
                }`}
                type="text"
                value={keepOnlyFilesPattern}
                disabled={isPreviewing}
                aria-invalid={Boolean(keepOnlyFilesPatternError)}
                aria-describedby={
                  keepOnlyFilesPatternError
                    ? "clean-files-pattern-error"
                    : undefined
                }
                onChange={(event) =>
                  setKeepOnlyFilesPattern(event.currentTarget.value)
                }
              />
              {keepOnlyFilesPatternError && (
                <p className="settings-error" id="clean-files-pattern-error">
                  {keepOnlyFilesPatternError}
                </p>
              )}
              {isPreviewing && (
                <p className="scan-wizard-muted">Scanning files...</p>
              )}
              {cleanStatus === "error" && (
                <p className="scan-wizard-error">{cleanError}</p>
              )}
            </div>
          )}

          {isDeleting && <p className="scan-wizard-muted">Deleting files...</p>}

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
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onCancel}
            disabled={isPreviewing || isDeleting}
          >
            Cancel
          </button>
          <button
            className="primary-btn"
            type="button"
            onClick={() =>
              isDeleteStep
                ? onConfirmDelete()
                : onPreview(keepOnlyFilesPattern)
            }
            disabled={
              isPreviewing || isDeleting || Boolean(keepOnlyFilesPatternError)
            }
          >
            {isPreviewing
              ? "Scanning..."
              : isDeleting
                ? "Deleting..."
                : "Confirm"}
          </button>
        </div>
      </section>
    </div>
  );
}
