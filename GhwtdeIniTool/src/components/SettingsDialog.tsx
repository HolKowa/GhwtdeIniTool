import {
  DEFAULT_KEEP_ONLY_FILES_PATTERN,
  type SettingsStatus,
} from "../hooks/useProjectSettings";
import type { ProjectSettings } from "../types/projectSettings";

type SettingsDialogProps = {
  canClose: boolean;
  onChangeCategoriesExtraFolder: () => void;
  onChangeFolder: () => void;
  onClearCategoriesExtraFolder: () => void;
  onClose: () => void;
  onKeepOnlyFilesPatternChange: (pattern: string) => void;
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
  settings,
  settingsError,
  settingsStatus,
}: SettingsDialogProps) {
  const hasAvailableModsFolder =
    settings?.mods_dir && settings.mods_dir_available;
  const hasAvailableCategoriesExtraFolder =
    settings?.categories_extra_dir && settings.categories_extra_dir_available;
  const canEditProjectSettings = Boolean(hasAvailableModsFolder);

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
              disabled={
                !canEditProjectSettings || !settings?.categories_extra_dir
              }
            >
              Clear folder
            </button>
            <button
              className="primary-btn"
              type="button"
              onClick={onChangeCategoriesExtraFolder}
              disabled={!canEditProjectSettings}
            >
              Change folder
            </button>
          </div>
        </div>

        <div className="settings-field">
          <div className="settings-field-header">
            <label htmlFor="keep-files-pattern">
              Keep only files with pattern
            </label>
            <div className="settings-actions">
              <button
                className="secondary-btn"
                type="button"
                onClick={() => onKeepOnlyFilesPatternChange("*")}
                disabled={!canEditProjectSettings}
              >
                Keep all
              </button>
              <button
                className="primary-btn"
                type="button"
                onClick={() =>
                  onKeepOnlyFilesPatternChange(DEFAULT_KEEP_ONLY_FILES_PATTERN)
                }
                disabled={!canEditProjectSettings}
              >
                Keep default
              </button>
            </div>
          </div>
          <input
            id="keep-files-pattern"
            className="settings-input"
            type="text"
            value={settings?.keep_only_files_pattern ?? ""}
            disabled={!canEditProjectSettings}
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
