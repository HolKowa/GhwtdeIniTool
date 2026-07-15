import type { UpdateStatus } from "../hooks/useAppUpdater";

type UpdateDialogProps = {
  downloadProgress: number;
  errorMessage: string;
  onDisableStartupChecks: () => void;
  onInstall: () => void;
  onSkip: () => void;
  onStartDownload: () => void;
  updateStatus: UpdateStatus;
};

export function UpdateDialog({
  downloadProgress,
  errorMessage,
  onDisableStartupChecks,
  onInstall,
  onSkip,
  onStartDownload,
  updateStatus,
}: UpdateDialogProps) {
  return (
    <div className="update-popup-backdrop" role="presentation">
      <div
        className={`update-popup ${updateStatus}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-popup-title"
      >
        {updateStatus === "available" && (
          <>
            <h2 id="update-popup-title">An update is available</h2>
            <p>A newer version of GhwtdeIniTool is ready to download.</p>
            <div className="update-buttons">
              <button className="update-now-btn" onClick={onStartDownload}>
                Update
              </button>
              <button className="skip-btn" onClick={onSkip}>
                Skip
              </button>
              <button className="skip-btn" onClick={onDisableStartupChecks}>
                Don't check again
              </button>
            </div>
          </>
        )}

        {updateStatus === "downloading" && (
          <>
            <h2 id="update-popup-title">Downloading update</h2>
            <p>{downloadProgress}% complete</p>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${downloadProgress}%` }}
              />
            </div>
          </>
        )}

        {updateStatus === "ready" && (
          <>
            <h2 id="update-popup-title">Update downloaded</h2>
            <p>Restart GhwtdeIniTool to install the update.</p>
            <button className="install-btn" onClick={onInstall}>
              Restart & Install
            </button>
          </>
        )}

        {updateStatus === "error" && (
          <>
            <h2 id="update-popup-title">Update failed</h2>
            <p>{errorMessage}</p>
            <button className="skip-btn" onClick={onSkip}>
              Close
            </button>
          </>
        )}
      </div>
    </div>
  );
}
