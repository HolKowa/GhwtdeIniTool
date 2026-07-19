import { useState } from "react";

import type { FolderSanitizeStatus, OfficialCategoryStatus } from "../hooks/useModsScanner";
import type { FolderSanitizeScanResult, OfficialCategoryScanResult } from "../types/scanMods";

type FixModsFolderWizardProps = {
  error: string;
  folderError: string;
  folderResult: FolderSanitizeScanResult | null;
  folderStatus: FolderSanitizeStatus;
  onClose: () => void;
  onCopyPath: (path: string) => void;
  onDisableAll: () => Promise<void>;
  onDisable: (relativePath: string) => Promise<void>;
  onEnable: (relativePath: string) => Promise<void>;
  onScanFolders: () => Promise<void>;
  onSanitizeFolders: (relativePaths: string[]) => Promise<void>;
  onVerifyAll: () => Promise<void>;
  result: OfficialCategoryScanResult | null;
  status: OfficialCategoryStatus;
};

export function FixModsFolderWizard({ error, folderError, folderResult, folderStatus, onClose, onCopyPath, onDisableAll, onDisable, onEnable, onScanFolders, onSanitizeFolders, onVerifyAll, result, status }: FixModsFolderWizardProps) {
  const [step, setStep] = useState<1 | 2>(1);
  const isCategoryBusy = status === "scanning" || status === "disabling" || status === "enabling";
  const isFolderBusy = folderStatus === "scanning" || folderStatus === "sanitizing";
  const isBusy = isCategoryBusy || isFolderBusy;
  const categories = result?.categories ?? [];
  const activeCount = categories.filter((category) => !category.is_disabled).length;
  const folders = folderResult?.folders ?? [];
  const canContinue = Boolean(result) && activeCount === 0 && !isBusy;
  const canFinish = Boolean(folderResult) && folders.length === 0 && !isBusy;

  const goToFolderStep = async () => {
    if (!canContinue) return;
    setStep(2);
    await onScanFolders();
  };

  return <div className="scan-wizard-backdrop" role="presentation"><section className="scan-wizard-panel" role="dialog" aria-modal="true" aria-labelledby="fix-mods-folder-title">
    <div className="scan-wizard-header"><div><p className="scan-wizard-step">Step {step} of 2</p><h2 id="fix-mods-folder-title">Fix MODS folder</h2></div><button className="close-btn" type="button" onClick={onClose} disabled={isBusy} aria-label="Close Fix MODS folder wizard">&times;</button></div>
    <div className="scan-wizard-body">
      {step === 1 ? <>
        {isCategoryBusy && <p className="scan-wizard-summary">Verifying categories...</p>}
        {error && <p className="scan-wizard-error">{error}</p>}
        {result && <><div className="content-issue-summary-row"><p className="scan-wizard-summary">{activeCount} active category {activeCount === 1 ? "needs" : "need"} attention. Disable every active category to continue.</p><div className="song-conflict-actions"><button className="secondary-btn" type="button" onClick={() => void onVerifyAll()} disabled={isBusy}>Verify all</button><button className="secondary-btn" type="button" onClick={() => void onDisableAll()} disabled={isBusy || activeCount === 0}>Disable all</button></div></div>
          {result.errors.map((scanError, index) => <p className="scan-wizard-error" key={`${index}-${scanError}`}>{scanError}</p>)}
          <div className="content-issue-groups">{categories.map((category) => <section className={`song-conflict-group${category.is_disabled ? " song-content-issue-group-disabled" : ""}`} key={category.relative_path}><div className="song-conflict-group-header"><span>{category.relative_path}</span><div className="song-conflict-actions">{category.is_disabled && <span className="song-content-issue-status resolved">Resolved</span>}<button className="secondary-btn" type="button" onClick={() => onCopyPath(category.folder_absolute_path)} disabled={isBusy}>Copy path</button><button className="secondary-btn" type="button" onClick={() => void (category.is_disabled ? onEnable(category.relative_path.replace(/category\.disabled\.ini$/i, "category.ini")) : onDisable(category.relative_path))} disabled={isBusy}>{category.is_disabled ? "Enable" : "Disable"}</button></div></div><ul className="song-conflict-list">{category.validation_reasons.map((reason) => <li className="song-content-issue-row song-content-issue-row-error" key={reason}><span><strong>{reason}</strong></span></li>)}</ul></section>)}</div>
          {categories.length === 0 && <p className="scan-wizard-muted">No official category collisions or incomplete CategoryInfo files were found.</p>}</>}
      </> : <>
        {isFolderBusy && <p className="scan-wizard-summary">Scanning folder names...</p>}
        {folderError && <p className="scan-wizard-error">{folderError}</p>}
        {folderResult && <><div className="content-issue-summary-row"><p className="scan-wizard-summary">{folders.length} illegal folder {folders.length === 1 ? "name needs" : "names need"} sanitizing.</p><div className="song-conflict-actions"><button className="secondary-btn" type="button" onClick={() => void onScanFolders()} disabled={isBusy}>Verify all</button><button className="secondary-btn" type="button" onClick={() => void onSanitizeFolders(folders.map((folder) => folder.relative_path))} disabled={isBusy || folders.length === 0}>Sanitize all</button></div></div>
          {folderResult.errors.map((scanError, index) => <p className="scan-wizard-error" key={`${index}-${scanError}`}>{scanError}</p>)}
          <div className="content-issue-groups">{folders.map((folder) => <section className="song-conflict-group" key={folder.relative_path}><div className="song-conflict-group-header folder-sanitize-header"><span className="folder-sanitize-path"><strong>Illegal folder:</strong><span className="folder-sanitize-path-value">{folder.relative_path}</span></span><div className="song-conflict-actions"><button className="secondary-btn" type="button" onClick={() => onCopyPath(folder.absolute_path)} disabled={isBusy}>Copy path</button></div></div><ul className="song-conflict-list"><li className="song-content-issue-row folder-sanitize-rename-row"><span className="folder-sanitize-path"><strong>Rename to:</strong><span className="folder-sanitize-path-value">{folder.sanitized_relative_path}</span>{folder.requires_collision_suffix && <small>A collision suffix was added.</small>}</span></li></ul></section>)}</div>
          {folders.length === 0 && <p className="scan-wizard-muted">No illegal folder names were found.</p>}</>}
      </>}
    </div>
    <div className="scan-wizard-actions">{step === 1 ? <button className="primary-btn" type="button" onClick={() => void goToFolderStep()} disabled={!canContinue}>Next</button> : <><button className="secondary-btn" type="button" onClick={() => setStep(1)} disabled={isBusy}>Back</button><button className="primary-btn" type="button" onClick={onClose} disabled={!canFinish}>Finish</button></>}</div>
  </section></div>;
}
