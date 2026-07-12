import { useEffect, useMemo, useState } from "react";

import type { GameIconCategoryStatus } from "../hooks/useModsScanner";
import type {
  GameIconCategoryScanResult,
  GameIconSongFixInput,
  GameIconSongFixPreview,
} from "../types/scanMods";

type FixGameIconsWizardProps = {
  error: string;
  fixPreview: GameIconSongFixPreview | null;
  onApplyFixes: (fixes: GameIconSongFixInput[]) => Promise<void>;
  onClose: () => void;
  onFix: () => void;
  onPreviewFixes: () => Promise<void>;
  result: GameIconCategoryScanResult | null;
  status: GameIconCategoryStatus;
};

type WizardStep = 1 | 2;

export function FixGameIconsWizard({
  error,
  fixPreview,
  onApplyFixes,
  onClose,
  onFix,
  onPreviewFixes,
  result,
  status,
}: FixGameIconsWizardProps) {
  const [step, setStep] = useState<WizardStep>(1);
  const [editedGameIcons, setEditedGameIcons] = useState<
    Record<string, string>
  >({});
  const isBusy =
    status === "scanning" ||
    status === "fixing" ||
    status === "previewing" ||
    status === "applying";
  const isError = status === "error";
  const canFix = Boolean(result?.needs_fix) && !isBusy;
  const canContinue = Boolean(result) && !result?.needs_fix && !isBusy;
  const nextTitle = result?.needs_fix
    ? "Fix categories before continuing"
    : "Scan GameIcons before continuing";
  const validGameIconSet = useMemo(
    () =>
      new Set(
        (fixPreview?.valid_game_icons ?? []).map((gameIcon) =>
          gameIcon.toLocaleLowerCase(),
        ),
      ),
    [fixPreview],
  );
  const fixRows = fixPreview?.rows ?? [];
  const hasInvalidEditedGameIcon = fixRows.some((row) => {
    const value = editedGameIcons[row.relative_path]?.trim() ?? "";

    return !validGameIconSet.has(value.toLocaleLowerCase());
  });
  const canApply =
    step === 2 &&
    !isBusy &&
    Boolean(fixPreview) &&
    !hasInvalidEditedGameIcon;

  useEffect(() => {
    if (!fixPreview) {
      setEditedGameIcons({});
      return;
    }

    setEditedGameIcons(
      Object.fromEntries(
        fixPreview.rows.map((row) => [row.relative_path, row.new_game_icon]),
      ),
    );
  }, [fixPreview]);

  const goToStep2 = async () => {
    if (!canContinue) {
      return;
    }

    setStep(2);
    await onPreviewFixes();
  };

  const applyFixes = async () => {
    if (!canApply) {
      return;
    }
    if (fixRows.length === 0) {
      onClose();
      return;
    }

    try {
      await onApplyFixes(
        fixRows.map((row) => ({
          relative_path: row.relative_path,
          new_game_icon: editedGameIcons[row.relative_path].trim(),
        })),
      );
    } catch {
      // The hook stores the backend error for the wizard to display.
    }
  };

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
            <p className="scan-wizard-step">Step {step} of 2</p>
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

        <div
          className={`scan-wizard-body${step === 2 ? " game-icon-step-two-body" : ""}`}
        >
          {step === 1 ? (
            <>
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
                    <p
                      className="scan-wizard-error"
                      key={`${index}-${scanError}`}
                    >
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
            </>
          ) : (
            <>
              {status === "previewing" && (
                <p className="scan-wizard-summary">
                  Preparing song GameIcon fixes...
                </p>
              )}
              {status === "applying" && (
                <p className="scan-wizard-summary">
                  Applying song GameIcon fixes...
                </p>
              )}
              {isError && <p className="scan-wizard-error">{error}</p>}

              {fixPreview && fixRows.length === 0 && (
                <p className="scan-wizard-muted">
                  No invalid song GameIcons were found.
                </p>
              )}

              {fixRows.length > 0 && (
                <>
                  <p className="scan-wizard-summary game-icon-summary">
                    {fixRows.length} songs have invalid GameIcons.
                  </p>
                  <div className="game-icon-song-fix-table-wrap">
                    <table className="game-icon-song-fix-table">
                      <thead>
                        <tr>
                          <th>Artist</th>
                          <th>Title</th>
                          <th>Invalid GameIcon</th>
                          <th>New GameIcon</th>
                        </tr>
                      </thead>
                      <tbody>
                        {fixRows.map((row) => {
                          const value = editedGameIcons[row.relative_path] ?? "";
                          const isInvalid =
                            !validGameIconSet.has(value.trim().toLocaleLowerCase());

                          return (
                            <tr key={row.relative_path}>
                              <td>{row.artist}</td>
                              <td>{row.title}</td>
                              <td>{row.invalid_game_icon}</td>
                              <td>
                                <input
                                  aria-label={`New GameIcon for ${row.relative_path}`}
                                  className={isInvalid ? "input-error" : ""}
                                  list="game-icon-valid-options"
                                  type="text"
                                  value={value}
                                  onChange={(event) =>
                                    setEditedGameIcons((currentValues) => ({
                                      ...currentValues,
                                      [row.relative_path]: event.target.value,
                                    }))
                                  }
                                  disabled={isBusy}
                                />
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                  <datalist id="game-icon-valid-options">
                    {(fixPreview?.valid_game_icons ?? []).map((gameIcon) => (
                      <option key={gameIcon} value={gameIcon} />
                    ))}
                  </datalist>
                </>
              )}
            </>
          )}
        </div>

        <div className="scan-wizard-actions">
          {step === 1 ? (
            <>
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
                onClick={goToStep2}
                disabled={!canContinue}
                title={canContinue ? undefined : nextTitle}
              >
                Next
              </button>
            </>
          ) : (
            <>
              <button
                className="secondary-btn"
                type="button"
                onClick={() => setStep(1)}
                disabled={isBusy}
              >
                Back
              </button>
              <button
                className="primary-btn"
                type="button"
                onClick={applyFixes}
                disabled={!canApply}
              >
                {status === "applying" ? "Applying..." : "Apply New and finish"}
              </button>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
