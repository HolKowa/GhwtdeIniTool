import type { SettingsStatus } from "../hooks/useProjectSettings";
import type { ProjectSettings } from "../types/projectSettings";

type SettingsDialogProps = {
  canClose: boolean;
  onChangeGameLogosFolder: () => void;
  onChangeFolder: () => void;
  onClose: () => void;
  onKeepOriginalSongIniChange: (keepOriginalSongIni: boolean) => void;
  settings: ProjectSettings | null;
  settingsError: string;
  settingsStatus: SettingsStatus;
};

export function SettingsDialog({
  canClose,
  onChangeGameLogosFolder,
  onChangeFolder,
  onClose,
  onKeepOriginalSongIniChange,
  settings,
  settingsError,
  settingsStatus,
}: SettingsDialogProps) {
  const hasAvailableModsFolder = Boolean(
    settings?.mods_dir && settings.mods_dir_available,
  );
  const hasAvailableGameLogosFolder = Boolean(
    settings?.official_gamelogos_dir &&
      settings.official_gamelogos_dir_available,
  );
  const isLoading = settingsStatus === "loading";

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
            <h3>Official GAMELOGOS folder</h3>
            <p
              className={
                hasAvailableGameLogosFolder
                  ? "folder-path"
                  : "folder-path muted"
              }
            >
              {settingsStatus === "loading"
                ? "Loading..."
                : settings?.official_gamelogos_dir ?? "No folder selected"}
            </p>
          </div>

          <button
            className="primary-btn"
            type="button"
            onClick={onChangeGameLogosFolder}
            disabled={isLoading || !hasAvailableModsFolder}
          >
            Change folder
          </button>
        </div>

        <label className="settings-toggle-row">
          <span>Keep original on first song.ini change</span>
          <input
            type="checkbox"
            checked={settings?.keep_original_song_ini ?? true}
            disabled={
              isLoading ||
              !hasAvailableModsFolder ||
              !hasAvailableGameLogosFolder
            }
            onChange={(event) =>
              onKeepOriginalSongIniChange(event.currentTarget.checked)
            }
          />
        </label>

        {settingsError && <p className="settings-error">{settingsError}</p>}
      </section>
    </div>
  );
}
