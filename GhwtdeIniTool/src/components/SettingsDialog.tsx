import type { SettingsStatus } from "../hooks/useProjectSettings";
import type { ProjectSettings } from "../types/projectSettings";

type SettingsDialogProps = {
  canClose: boolean;
  onChangeFolder: () => void;
  onClose: () => void;
  settings: ProjectSettings | null;
  settingsError: string;
  settingsStatus: SettingsStatus;
};

export function SettingsDialog({
  canClose,
  onChangeFolder,
  onClose,
  settings,
  settingsError,
  settingsStatus,
}: SettingsDialogProps) {
  const hasAvailableModsFolder =
    settings?.mods_dir && settings.mods_dir_available;

  return (
    <div className="settings-backdrop" role="presentation">
      <section
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <div className="settings-header">
          <h2 id="settings-title">Settings</h2>

          {canClose && (
            <button
              className="close-btn"
              type="button"
              onClick={onClose}
              aria-label="Close settings"
            >
              &times;
            </button>
          )}
        </div>

        <div className="settings-row">
          <div>
            <h3>MODS folder</h3>
            <p
              className={
                hasAvailableModsFolder ? "folder-path" : "folder-path muted"
              }
            >
              {settingsStatus === "loading"
                ? "Loading..."
                : settings?.mods_dir ?? "No folder selected"}
            </p>
          </div>

          <button
            className="primary-btn"
            type="button"
            onClick={onChangeFolder}
          >
            Change folder
          </button>
        </div>

        {settingsError && <p className="settings-error">{settingsError}</p>}
      </section>
    </div>
  );
}
