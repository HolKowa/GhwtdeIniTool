import type { OfficialCategoryStatus } from "../hooks/useModsScanner";
import type { OfficialCategoryScanResult } from "../types/scanMods";

type FixModsFolderWizardProps = {
  error: string;
  onClose: () => void;
  onCopyPath: (path: string) => void;
  onDisableAll: () => Promise<void>;
  onDisable: (relativePath: string) => Promise<void>;
  onEnable: (relativePath: string) => Promise<void>;
  onVerifyAll: () => Promise<void>;
  result: OfficialCategoryScanResult | null;
  status: OfficialCategoryStatus;
};

export function FixModsFolderWizard({
  error,
  onClose,
  onCopyPath,
  onDisableAll,
  onDisable,
  onEnable,
  onVerifyAll,
  result,
  status,
}: FixModsFolderWizardProps) {
  const isBusy = status === "scanning" || status === "disabling" || status === "enabling";
  const categories = result?.categories ?? [];
  const activeCount = categories.filter((category) => !category.is_disabled).length;
  const canContinue = Boolean(result) && activeCount === 0 && !isBusy;

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section className="scan-wizard-panel" role="dialog" aria-modal="true" aria-labelledby="fix-mods-folder-title">
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">Step 1 of 2</p>
            <h2 id="fix-mods-folder-title">Fix MODS folder</h2>
          </div>
          <button className="close-btn" type="button" onClick={onClose} disabled={isBusy} aria-label="Close Fix MODS folder wizard">&times;</button>
        </div>
        <div className="scan-wizard-body">
          {isBusy && <p className="scan-wizard-summary">Verifying categories...</p>}
          {error && <p className="scan-wizard-error">{error}</p>}
          {result && (
            <>
              <div className="content-issue-summary-row">
                <p className="scan-wizard-summary">
                  {activeCount} active official category {activeCount === 1 ? "collision" : "collisions"}. Disable every active category to continue.
                </p>
                <div className="song-conflict-actions">
                  <button className="secondary-btn" type="button" onClick={() => void onVerifyAll()} disabled={isBusy}>
                    Verify all
                  </button>
                  <button className="secondary-btn" type="button" onClick={() => void onDisableAll()} disabled={isBusy || activeCount === 0}>
                    Disable all
                  </button>
                </div>
              </div>
              {result.errors.map((scanError, index) => <p className="scan-wizard-error" key={`${index}-${scanError}`}>{scanError}</p>)}
              <div className="content-issue-groups">
                {categories.map((category) => (
                  <section className={`song-conflict-group${category.is_disabled ? " song-content-issue-group-disabled" : ""}`} key={category.relative_path}>
                    <div className="song-conflict-group-header">
                      <span>{category.relative_path}</span>
                      <div className="song-conflict-actions">
                        {category.is_disabled && <span className="song-content-issue-status resolved">Resolved</span>}
                        <button className="secondary-btn" type="button" onClick={() => onCopyPath(category.folder_absolute_path)} disabled={isBusy}>Copy path</button>
                        <button className="secondary-btn" type="button" onClick={() => void (category.is_disabled ? onEnable(category.relative_path.replace(/category\.disabled\.ini$/i, "category.ini")) : onDisable(category.relative_path))} disabled={isBusy}>
                          {category.is_disabled ? "Enable" : "Disable"}
                        </button>
                      </div>
                    </div>
                    <ul className="song-conflict-list"><li><span>Checksum: {category.checksum}</span></li></ul>
                  </section>
                ))}
              </div>
              {categories.length === 0 && <p className="scan-wizard-muted">No categories match an official GameLogo checksum.</p>}
            </>
          )}
        </div>
        <div className="scan-wizard-actions">
          <button className="primary-btn" type="button" disabled={!canContinue}>Next</button>
        </div>
      </section>
    </div>
  );
}
