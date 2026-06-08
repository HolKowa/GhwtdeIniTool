import type { ScanModsStatus } from "../hooks/useModsScanner";
import type { ScanModsPreview } from "../types/scanMods";

type ScanWizardProps = {
  onCancel: () => void;
  onConfirm: () => void;
  preview: ScanModsPreview | null;
  scanError: string;
  scanStatus: ScanModsStatus;
};

export function ScanWizard({
  onCancel,
  onConfirm,
  preview,
  scanError,
  scanStatus,
}: ScanWizardProps) {
  const isPreviewing = scanStatus === "previewing";
  const isMoving = scanStatus === "moving";
  const hasFilesToMove = Boolean(preview?.files_to_move.length);

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
            <p className="scan-wizard-step">Step 1</p>
            <h2 id="scan-wizard-title">Move categories</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onCancel}
            aria-label="Cancel scan"
            disabled={isMoving}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {isPreviewing && <p className="scan-wizard-muted">Scanning...</p>}

          {scanStatus === "error" && (
            <p className="scan-wizard-error">{scanError}</p>
          )}

          {preview && (
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
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onCancel}
            disabled={isMoving}
          >
            Cancel
          </button>
          <button
            className="primary-btn"
            type="button"
            onClick={onConfirm}
            disabled={isPreviewing || isMoving || scanStatus === "error"}
          >
            {isMoving ? "Moving..." : "Confirm"}
          </button>
        </div>
      </section>
    </div>
  );
}
