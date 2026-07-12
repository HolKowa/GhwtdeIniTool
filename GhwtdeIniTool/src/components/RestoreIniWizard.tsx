import { useState } from "react";

import type { RestoreIniStatus } from "../hooks/useIniRestorer";
import type { RestoreIniAction } from "../types/scanMods";

type RestoreIniWizardProps = {
  error: string;
  errors: string[];
  onCancel: () => void;
  onConfirm: (action: RestoreIniAction) => void;
  status: RestoreIniStatus;
};

export function RestoreIniWizard({
  error,
  errors,
  onCancel,
  onConfirm,
  status,
}: RestoreIniWizardProps) {
  const [action, setAction] = useState<RestoreIniAction>(
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
              Choose INI action
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
                name="restore-ini-action"
                value="beforeFormatIssueFix"
                checked={action === "beforeFormatIssueFix"}
                disabled={isRestoring}
                onChange={() => setAction("beforeFormatIssueFix")}
              />
              <span>Restore original song.ini (before format fixes)</span>
            </label>
            <label className="restore-mode-option">
              <input
                type="radio"
                name="restore-ini-action"
                value="afterFormatIssueFixes"
                checked={action === "afterFormatIssueFixes"}
                disabled={isRestoring}
                onChange={() => setAction("afterFormatIssueFixes")}
              />
              <span>Restore original song.ini (after format fixes)</span>
            </label>
            <label className="restore-mode-option">
              <input
                type="radio"
                name="restore-ini-action"
                value="deleteInstrumentSidecars"
                checked={action === "deleteInstrumentSidecars"}
                disabled={isRestoring}
                onChange={() => setAction("deleteInstrumentSidecars")}
              />
              <span>Delete song.instruments.ini files</span>
            </label>
          </div>

          {isRestoring && (
            <p className="scan-wizard-muted">Processing INI files...</p>
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
            className="primary-btn"
            type="button"
            onClick={() => onConfirm(action)}
            disabled={isRestoring}
          >
            {isRestoring ? "Processing..." : "Run action"}
          </button>
        </div>
      </section>
    </div>
  );
}
