import type { SettingsStatus } from "../hooks/useProjectSettings";
import type { ProjectSettings } from "../types/projectSettings";

type SettingsDialogProps = {
  canClose: boolean;
  onChangeCategoriesExtraFolder: () => void;
  onChangeFolder: () => void;
  onClearCategoriesExtraFolder: () => void;
  onClose: () => void;
  onKeepOnlyFilesPatternChange: (pattern: string) => void;
  onKeepOnlyFilesWithPatternChange: (enabled: boolean) => void;
  settings: ProjectSettings | null;
  settingsError: string;
  settingsStatus: SettingsStatus;
};

export function SettingsDialog({
  canClose,
  onChangeCategoriesExtraFolder,
  onChangeFolder,
  onClearCategoriesExtraFolder,
  onClose,
  onKeepOnlyFilesPatternChange,
  onKeepOnlyFilesWithPatternChange,
  settings,
  settingsError,
  settingsStatus,
}: SettingsDialogProps) {
  const hasAvailableModsFolder =
    settings?.mods_dir && settings.mods_dir_available;
  const hasAvailableCategoriesExtraFolder =
    settings?.categories_extra_dir && settings.categories_extra_dir_available;

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

        <div className="settings-row">
          <div>
            <h3>Move categories to extra folder</h3>
            <p
              className={
                hasAvailableCategoriesExtraFolder
                  ? "folder-path"
                  : "folder-path muted"
              }
            >
              {settingsStatus === "loading"
                ? "Loading..."
                : settings?.categories_extra_dir ?? "No folder selected"}
            </p>
          </div>

          <div className="settings-actions">
            <button
              className="secondary-btn"
              type="button"
              onClick={onClearCategoriesExtraFolder}
              disabled={!settings?.categories_extra_dir}
            >
              Clear folder
            </button>
            <button
              className="primary-btn"
              type="button"
              onClick={onChangeCategoriesExtraFolder}
            >
              Change folder
            </button>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-check-row">
            <span>Keep only files with pattern</span>
            <input
              type="checkbox"
              checked={settings?.keep_only_files_with_pattern ?? false}
              onChange={(event) =>
                onKeepOnlyFilesWithPatternChange(event.currentTarget.checked)
              }
            />
          </label>
          <input
            className="settings-input"
            type="text"
            value={settings?.keep_only_files_pattern ?? ""}
            onChange={(event) =>
              onKeepOnlyFilesPatternChange(event.currentTarget.value)
            }
          />
        </div>

        {settingsError && <p className="settings-error">{settingsError}</p>}
      </section>
    </div>
  );
}
