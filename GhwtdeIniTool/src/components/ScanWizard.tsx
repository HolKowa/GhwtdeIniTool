import type { ScanModsStatus } from "../hooks/useModsScanner";
import type { DeleteFilesPreview, ScanModsPreview } from "../types/scanMods";

type ScanWizardProps = {
  deletePreview: DeleteFilesPreview | null;
  onCancel: () => void;
  onConfirm: () => void;
  preview: ScanModsPreview | null;
  scanError: string;
  scanStatus: ScanModsStatus;
};

export function ScanWizard({
  deletePreview,
  onCancel,
  onConfirm,
  preview,
  scanError,
  scanStatus,
}: ScanWizardProps) {
  const isPreviewing = scanStatus === "previewing";
  const isMoving = scanStatus === "moving";
  const isDeleting = scanStatus === "deleting";
  const isBusy = isPreviewing || isMoving || isDeleting;
  const isDeleteStep = scanStatus === "readyDelete" || isDeleting;
  const hasFilesToMove = Boolean(preview?.files_to_move.length);
  const hasFilesToDelete = Boolean(deletePreview?.files_to_delete.length);

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
              {isDeleteStep ? "Step 2" : "Step 1"}
            </p>
            <h2 id="scan-wizard-title">
              {isDeleteStep ? "Keep only files with pattern" : "Move categories"}
            </h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onCancel}
            aria-label="Cancel scan"
            disabled={isMoving || isDeleting}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {isPreviewing && <p className="scan-wizard-muted">Scanning...</p>}
          {isMoving && (
            <p className="scan-wizard-muted">
              Moving categories and preparing delete preview...
            </p>
          )}
          {isDeleting && <p className="scan-wizard-muted">Deleting files...</p>}

          {scanStatus === "error" && (
            <p className="scan-wizard-error">{scanError}</p>
          )}

          {!isDeleteStep && preview && (
            <>
              <p className="scan-wizard-summary">
                {preview.files_to_move.length} files ready to move from{" "}
                {preview.categories_found} categories.
              </p>

              {!preview.moved_categories_enabled && (
                <p className="scan-wizard-muted">
                  Category moving is disabled because no extra folder is
                  available.
                </p>
              )}

              {hasFilesToMove ? (
                <ul className="scan-file-list">
                  {preview.files_to_move.map((filePath) => (
                    <li key={filePath}>{filePath}</li>
                  ))}
                </ul>
              ) : (
                <p className="scan-wizard-muted">No files will be moved.</p>
              )}

              {preview.errors.map((error, index) => (
                <p className="scan-wizard-error" key={`${index}-${error}`}>
                  {error}
                </p>
              ))}
            </>
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
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onCancel}
            disabled={isMoving || isDeleting}
          >
            Cancel
          </button>
          <button
            className="primary-btn"
            type="button"
            onClick={onConfirm}
            disabled={isBusy || scanStatus === "error"}
          >
            {isMoving
              ? "Moving..."
              : isDeleting
                ? "Deleting..."
                : "Confirm"}
          </button>
        </div>
      </section>
    </div>
  );
}
