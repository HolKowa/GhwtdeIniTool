import { useEffect, useState } from "react";

import {
  DEFAULT_KEEP_ONLY_FILES_PATTERN,
  type SettingsStatus,
} from "../hooks/useProjectSettings";
import type { ProjectSettings } from "../types/projectSettings";

type SettingsDialogProps = {
  canClose: boolean;
  onChangeFolder: () => void;
  onClose: () => void;
  onKeepOnlyFilesPatternChange: (pattern: string) => void;
  settings: ProjectSettings | null;
  settingsError: string;
  settingsStatus: SettingsStatus;
};

export function SettingsDialog({
  canClose,
  onChangeFolder,
  onClose,
  onKeepOnlyFilesPatternChange,
  settings,
  settingsError,
  settingsStatus,
}: SettingsDialogProps) {
  const [keepOnlyFilesPatternInput, setKeepOnlyFilesPatternInput] = useState(
    settings?.keep_only_files_pattern ?? "",
  );
  const [isKeepOnlyFilesPatternDirty, setIsKeepOnlyFilesPatternDirty] =
    useState(false);
  const hasAvailableModsFolder =
    settings?.mods_dir && settings.mods_dir_available;
  const canEditProjectSettings = Boolean(hasAvailableModsFolder);
  const keepOnlyFilesPatternError = validateKeepOnlyFilesPattern(
    keepOnlyFilesPatternInput,
  );
  const canCloseSettings = canClose && !keepOnlyFilesPatternError;
  const savedKeepOnlyFilesPattern = settings?.keep_only_files_pattern ?? "";

  useEffect(() => {
    if (
      isKeepOnlyFilesPatternDirty &&
      keepOnlyFilesPatternInput !== savedKeepOnlyFilesPattern
    ) {
      return;
    }

    setKeepOnlyFilesPatternInput(savedKeepOnlyFilesPattern);
    setIsKeepOnlyFilesPatternDirty(false);
  }, [
    isKeepOnlyFilesPatternDirty,
    keepOnlyFilesPatternInput,
    savedKeepOnlyFilesPattern,
  ]);

  const updateKeepOnlyFilesPattern = (pattern: string) => {
    setKeepOnlyFilesPatternInput(pattern);
    setIsKeepOnlyFilesPatternDirty(true);

    if (!validateKeepOnlyFilesPattern(pattern)) {
      onKeepOnlyFilesPatternChange(pattern);
    }
  };

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
              disabled={!canCloseSettings}
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

        <div className="settings-field">
          <div className="settings-field-header">
            <label htmlFor="keep-files-pattern">
              Keep only files with pattern
            </label>
            <div className="settings-actions">
              <button
                className="secondary-btn"
                type="button"
                onClick={() => updateKeepOnlyFilesPattern("*")}
                disabled={!canEditProjectSettings}
              >
                Keep all
              </button>
              <button
                className="primary-btn"
                type="button"
                onClick={() =>
                  updateKeepOnlyFilesPattern(DEFAULT_KEEP_ONLY_FILES_PATTERN)
                }
                disabled={!canEditProjectSettings}
              >
                Keep default
              </button>
            </div>
          </div>
          <input
            id="keep-files-pattern"
            className={`settings-input${
              keepOnlyFilesPatternError ? " settings-input-error" : ""
            }`}
            type="text"
            value={keepOnlyFilesPatternInput}
            disabled={!canEditProjectSettings}
            aria-invalid={Boolean(keepOnlyFilesPatternError)}
            aria-describedby={
              keepOnlyFilesPatternError
                ? "keep-files-pattern-error"
                : undefined
            }
            onChange={(event) =>
              updateKeepOnlyFilesPattern(event.currentTarget.value)
            }
          />
          {keepOnlyFilesPatternError && (
            <p className="settings-error" id="keep-files-pattern-error">
              {keepOnlyFilesPatternError}
            </p>
          )}
        </div>

        {settingsError && <p className="settings-error">{settingsError}</p>}
      </section>
    </div>
  );
}

function validateKeepOnlyFilesPattern(pattern: string) {
  const invalidCharacterPattern = /[<>:"/\\|?]/;
  const invalidCharacters = '< > : " / \\ | ?';
  const patterns = pattern.split(",");

  for (const rawPattern of patterns) {
    const trimmedPattern = rawPattern.trim();

    if (!trimmedPattern) {
      continue;
    }

    if (invalidCharacterPattern.test(trimmedPattern)) {
      return `Keep patterns match file names only. Remove path or reserved characters: ${invalidCharacters}.`;
    }
  }

  return "";
}
