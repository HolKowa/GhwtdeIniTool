import type { GameIconCategoryStatus } from "../hooks/useModsScanner";
import type { GameIconCategoryScanResult } from "../types/scanMods";

type FixGameIconsWizardProps = {
  error: string;
  onClose: () => void;
  onFix: () => void;
  result: GameIconCategoryScanResult | null;
  status: GameIconCategoryStatus;
};

export function FixGameIconsWizard({
  error,
  onClose,
  onFix,
  result,
  status,
}: FixGameIconsWizardProps) {
  const isBusy = status === "scanning" || status === "fixing";
  const isError = status === "error";
  const canFix = Boolean(result?.needs_fix) && !isBusy;
  const nextTitle = result?.needs_fix
    ? "Fix categories before continuing"
    : "Next step is not implemented yet";

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel game-icon-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="game-icon-title"
      >
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">Step 1 of 2</p>
            <h2 id="game-icon-title">Fix GameIcons</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onClose}
            aria-label="Close GameIcons wizard"
            disabled={isBusy}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          <div className="game-icon-info">
            <p>In order to use custom GameIcons in song.ini files:</p>
            <ul>
              <li>gamelogo_*.img.xen must exist within MODS folder.</li>
              <li>
                next to gamelogo_*.img.xen there must be a category.ini with
                Logo=gamelogo_*
              </li>
            </ul>
            <p>Original GameIcons can be used out of the box:</p>
            <ul>
              <li>Located in DATA/IMAGES/GAMELOGOS.</li>
              <li>You cannot just add custom gamelogos in there.</li>
            </ul>
          </div>

          {status === "scanning" && (
            <p className="scan-wizard-summary">Scanning GameIcons...</p>
          )}
          {status === "fixing" && (
            <p className="scan-wizard-summary">Fixing categories...</p>
          )}
          {isError && <p className="scan-wizard-error">{error}</p>}

          {result && (
            <>
              <p className="scan-wizard-summary game-icon-summary">
                {result.custom_game_icons.length} custom GameIcons found
                {result.needs_fix
                  ? ", categories need attention."
                  : ", categories are ready."}
              </p>

              {result.errors.map((scanError, index) => (
                <p className="scan-wizard-error" key={`${index}-${scanError}`}>
                  {scanError}
                </p>
              ))}

              {result.groups.length === 0 ? (
                <p className="scan-wizard-muted">
                  No custom gamelogo_*.img.xen files were found in MODS.
                </p>
              ) : (
                <div className="game-icon-group-list">
                  {result.groups.map((group) => (
                    <section
                      className="game-icon-group"
                      key={group.folder_absolute_path}
                    >
                      <div className="game-icon-group-header">
                        <span>{group.folder_relative_path || "."}</span>
                        {group.has_multiple_gamelogos && (
                          <span className="game-icon-warning">
                            multiple gamelogos
                          </span>
                        )}
                      </div>
                      <ul className="game-icon-file-list">
                        {group.gamelogos.map((gamelogo) => (
                          <li key={gamelogo.relative_path}>
                            <span>{gamelogo.file_name}</span>
                            {gamelogo.has_matching_category_ini && (
                              <span className="game-icon-chip">
                                category.ini
                              </span>
                            )}
                          </li>
                        ))}
                      </ul>
                    </section>
                  ))}
                </div>
              )}
            </>
          )}
        </div>

        <div className="scan-wizard-actions">
          <button
            className="secondary-btn"
            type="button"
            onClick={onFix}
            disabled={!canFix}
          >
            {status === "fixing" ? "Fixing..." : "Fix Categories"}
          </button>
          <button
            className="primary-btn"
            type="button"
            disabled
            title={nextTitle}
          >
            Next
          </button>
        </div>
      </section>
    </div>
  );
}
