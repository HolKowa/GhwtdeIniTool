import { useState } from "react";

import type { RestoreIniStatus } from "../hooks/useIniRestorer";
import type { RestoreOriginalSongIniMode } from "../types/scanMods";

type RestoreIniWizardProps = {
  error: string;
  errors: string[];
  onCancel: () => void;
  onConfirm: (mode: RestoreOriginalSongIniMode) => void;
  status: RestoreIniStatus;
};

export function RestoreIniWizard({
  error,
  errors,
  onCancel,
  onConfirm,
  status,
}: RestoreIniWizardProps) {
  const [mode, setMode] = useState<RestoreOriginalSongIniMode>(
    "afterFormatIssueFixes",
  );
  const isRestoring = status === "restoring";

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel restore-ini-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="restore-ini-title"
      >
        <div className="scan-wizard-header">
          <div>
            <h2 id="restore-ini-title">
              Restore all song.ini files to original
            </h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onCancel}
            aria-label="Cancel restore"
            disabled={isRestoring}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          <div className="restore-mode-list">
            <label className="restore-mode-option">
              <input
                type="radio"
                name="restore-ini-mode"
                value="beforeFormatIssueFix"
                checked={mode === "beforeFormatIssueFix"}
                disabled={isRestoring}
                onChange={() => setMode("beforeFormatIssueFix")}
              />
              <span>Before format issue fix</span>
            </label>
            <label className="restore-mode-option">
              <input
                type="radio"
                name="restore-ini-mode"
                value="afterFormatIssueFixes"
                checked={mode === "afterFormatIssueFixes"}
                disabled={isRestoring}
                onChange={() => setMode("afterFormatIssueFixes")}
              />
              <span>After format issue fixes</span>
            </label>
          </div>

          {isRestoring && (
            <p className="scan-wizard-muted">Restoring song.ini files...</p>
          )}
          {status === "error" && (
            <div className="restore-error-details">
              <p className="scan-wizard-error">{error}</p>
              {errors.length > 0 && (
                <ul className="scan-file-list restore-error-list">
                  {errors.map((restoreError, index) => (
                    <li key={`${index}-${restoreError}`}>{restoreError}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onCancel}
            disabled={isRestoring}
          >
            Cancel
          </button>
          <button
            className="primary-btn"
            type="button"
            onClick={() => onConfirm(mode)}
            disabled={isRestoring}
          >
            {isRestoring ? "Restoring..." : "Finish"}
          </button>
        </div>
      </section>
    </div>
  );
}
