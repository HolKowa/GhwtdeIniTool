import { useEffect, useMemo, useRef, useState } from "react";

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
  onBack: () => void;
  onCancel: () => void;
  onConfirmDelete: (filesToDelete: string[]) => void;
  onPreview: (keepOnlyFilesPattern: string) => void;
};

type SortKey = "path" | "file";
type SortDirection = "asc" | "desc";

type DeleteFileRow = {
  file: string;
  path: string;
  relativePath: string;
};

function deleteFileRow(relativePath: string): DeleteFileRow {
  const separatorIndex = Math.max(
    relativePath.lastIndexOf("/"),
    relativePath.lastIndexOf("\\"),
  );

  return separatorIndex < 0
    ? { file: relativePath, path: "", relativePath }
    : {
        file: relativePath.slice(separatorIndex + 1),
        path: relativePath.slice(0, separatorIndex),
        relativePath,
      };
}

export function CleanModsWizard({
  cleanError,
  cleanStatus,
  deletePreview,
  onBack,
  onCancel,
  onConfirmDelete,
  onPreview,
}: CleanModsWizardProps) {
  const [keepOnlyFilesPattern, setKeepOnlyFilesPattern] = useState(
    DEFAULT_KEEP_ONLY_FILES_PATTERN,
  );
  const [selectedFilePaths, setSelectedFilePaths] = useState<Set<string>>(
    new Set(),
  );
  const [sortKey, setSortKey] = useState<SortKey>("path");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const selectAllRef = useRef<HTMLInputElement>(null);
  const keepOnlyFilesPatternError =
    validateKeepOnlyFilesPattern(keepOnlyFilesPattern);
  const isPreviewing = cleanStatus === "previewing";
  const isDeleting = cleanStatus === "deleting";
  const isDeleteStep = cleanStatus === "readyDelete" || isDeleting;
  const hasFilesToDelete = Boolean(deletePreview?.files_to_delete.length);
  const deleteFileRows = useMemo(
    () =>
      (deletePreview?.files_to_delete ?? [])
        .map(deleteFileRow)
        .sort((left, right) => {
          const primaryComparison = left[sortKey].localeCompare(right[sortKey]);
          const comparison =
            primaryComparison || left.relativePath.localeCompare(right.relativePath);

          return sortDirection === "asc" ? comparison : -comparison;
        }),
    [deletePreview, sortDirection, sortKey],
  );
  const selectedFileCount = selectedFilePaths.size;
  const areAllFilesSelected =
    deleteFileRows.length > 0 && selectedFileCount === deleteFileRows.length;
  const stepLabel = isDeleteStep ? "Step 2" : "Step 1";
  const title = isDeleteStep
    ? "Review files to delete"
    : "Keep only files with pattern";

  useEffect(() => {
    setSelectedFilePaths(new Set(deletePreview?.files_to_delete ?? []));
  }, [deletePreview]);

  useEffect(() => {
    if (selectAllRef.current) {
      selectAllRef.current.indeterminate =
        selectedFileCount > 0 && !areAllFilesSelected;
    }
  }, [areAllFilesSelected, selectedFileCount]);

  function changeSort(nextSortKey: SortKey) {
    if (nextSortKey === sortKey) {
      setSortDirection((currentDirection) =>
        currentDirection === "asc" ? "desc" : "asc",
      );
      return;
    }

    setSortKey(nextSortKey);
    setSortDirection("asc");
  }

  function toggleFileSelection(relativePath: string) {
    setSelectedFilePaths((currentSelectedFilePaths) => {
      const nextSelectedFilePaths = new Set(currentSelectedFilePaths);

      if (nextSelectedFilePaths.has(relativePath)) {
        nextSelectedFilePaths.delete(relativePath);
      } else {
        nextSelectedFilePaths.add(relativePath);
      }

      return nextSelectedFilePaths;
    });
  }

  function toggleAllFileSelections() {
    setSelectedFilePaths(
      areAllFilesSelected
        ? new Set()
        : new Set(deletePreview?.files_to_delete ?? []),
    );
  }

  function sortIndicator(column: SortKey) {
    return sortKey === column ? (sortDirection === "asc" ? "asc" : "desc") : "";
  }

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
            <div className="clean-delete-review">
              <p className="scan-wizard-summary">
                {selectedFileCount} of {deletePreview.files_to_delete.length} files
                do not match the keep patterns and are selected for deletion.
              </p>

              {hasFilesToDelete ? (
                <div className="clean-files-table-wrap">
                  <table className="clean-files-table">
                    <thead>
                      <tr>
                        <th className="clean-files-select-column">
                          <input
                            ref={selectAllRef}
                            aria-label="Select all files"
                            checked={areAllFilesSelected}
                            disabled={isDeleting}
                            type="checkbox"
                            onChange={toggleAllFileSelections}
                          />
                        </th>
                        {(["path", "file"] as const).map((column) => (
                          <th key={column}>
                            <button
                              type="button"
                              onClick={() => changeSort(column)}
                            >
                              {column === "path" ? "Path" : "File"}
                              <span aria-hidden="true">
                                {sortIndicator(column)}
                              </span>
                            </button>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {deleteFileRows.map((row) => (
                        <tr key={row.relativePath}>
                          <td className="clean-files-select-column">
                            <input
                              aria-label={`Select ${row.relativePath}`}
                              checked={selectedFilePaths.has(row.relativePath)}
                              disabled={isDeleting}
                              type="checkbox"
                              onChange={() => toggleFileSelection(row.relativePath)}
                            />
                          </td>
                          <td title={row.path}>{row.path || "—"}</td>
                          <td title={row.file}>{row.file}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p className="scan-wizard-muted">No files will be deleted.</p>
              )}

              {deletePreview.errors.map((error, index) => (
                <p className="scan-wizard-error" key={`${index}-${error}`}>
                  {error}
                </p>
              ))}
            </div>
          )}
        </div>

        <div className="scan-wizard-actions">
          {isDeleteStep && (
            <button
              className="secondary-btn"
              type="button"
              onClick={onBack}
              disabled={isDeleting}
            >
              Back
            </button>
          )}
          <button
            className="primary-btn"
            type="button"
            onClick={() =>
              isDeleteStep
                ? onConfirmDelete([...selectedFilePaths])
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
                : isDeleteStep
                  ? "Delete"
                  : "Next"}
          </button>
        </div>
      </section>
    </div>
  );
}
