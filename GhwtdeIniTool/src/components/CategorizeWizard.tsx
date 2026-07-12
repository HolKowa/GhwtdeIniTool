import { useEffect, useState } from "react";

import { previewIniToolCategories } from "../services/scanModsApi";

type CategorizeWizardProps = {
  hasActiveFilters: boolean;
  hasUnsavedSongChanges: boolean;
  includedSongCount: number;
  modsDir: string | null;
  onClose: () => void;
};

const songsPerCategory = 200;
const recommendedSongCap = 4000;

export function CategorizeWizard({
  hasActiveFilters,
  hasUnsavedSongChanges,
  includedSongCount,
  modsDir,
  onClose,
}: CategorizeWizardProps) {
  const [maximumSongCap, setMaximumSongCap] = useState(
    String(recommendedSongCap),
  );
  const [hasNonCategoryFiles, setHasNonCategoryFiles] = useState(false);
  const [categoriesPreviewError, setCategoriesPreviewError] = useState("");
  const parsedMaximumSongCap = Number(maximumSongCap);
  const hasValidMaximumSongCap =
    Number.isInteger(parsedMaximumSongCap) && parsedMaximumSongCap >= 1;
  const categorizedSongCount = hasValidMaximumSongCap
    ? Math.min(includedSongCount, parsedMaximumSongCap)
    : 0;
  const categoryCount = Math.ceil(categorizedSongCount / songsPerCategory);
  const excludedForCapCount = hasValidMaximumSongCap
    ? includedSongCount - categorizedSongCount
    : 0;
  const categoryFolder = `${modsDir ?? "MODS folder"}/IniToolCategories`;

  useEffect(() => {
    let isCurrent = true;

    previewIniToolCategories()
      .then((preview) => {
        if (!isCurrent) {
          return;
        }

        setHasNonCategoryFiles(preview.has_non_category_files);
      })
      .catch((err) => {
        if (isCurrent) {
          setCategoriesPreviewError(String(err));
        }
      });

    return () => {
      isCurrent = false;
    };
  }, []);

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel categorize-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="categorize-title"
      >
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">Step 1 of 2</p>
            <h2 id="categorize-title">Categorize songs</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={onClose}
            aria-label="Close categorize wizard"
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body categorize-wizard-body">
          <label className="categorize-cap-field">
            <span>Cap maximum included songs</span>
            <input
              aria-invalid={!hasValidMaximumSongCap}
              min="1"
              step="1"
              type="number"
              value={maximumSongCap}
              onChange={(event) => setMaximumSongCap(event.target.value)}
            />
          </label>
          {!hasValidMaximumSongCap && (
            <p className="scan-wizard-error">
              Enter a whole number of at least 1.
            </p>
          )}

          <ul className="categorize-bullet-list">
            <li
              className={`categorize-cap-advisory${
                hasValidMaximumSongCap && parsedMaximumSongCap > recommendedSongCap
                  ? " categorize-warning"
                  : ""
              }`}
            >
              Max 4000 songs are recommended. The game can fail when too many
              songs are loaded; larger libraries are at your own risk.
            </li>
            <li>
              Ordering uses the scanned songs list in the background. Change the
              table order before opening this wizard.
            </li>
            {excludedForCapCount > 0 && (
              <li>
                The last {excludedForCapCount} songs from the list are excluded to
                fit within maximum song cap.
              </li>
            )}
            <li>
              Included songs are linked into categories of {songsPerCategory} songs.
            </li>
            <li className={hasActiveFilters ? "categorize-warning" : undefined}>
              Filters of the scanned songs list are not used while linking songs.
            </li>
            <li
              className={
                hasUnsavedSongChanges ? "categorize-warning" : undefined
              }
            >
              Unsaved changes of songs are not saved while categorizing the songs.
            </li>
            <li>
              All songs not included are renamed to song.exclude.ini so they do not
              show as uncategorized in the game.
            </li>
            <li>All included songs get their GameCategory entry edited.</li>
            <li
              className={
                hasNonCategoryFiles ? "categorize-warning" : undefined
              }
            >
              All old files within this folder are deleted:
              <span className="categorize-folder-path">{categoryFolder}</span>
            </li>
            {hasValidMaximumSongCap && (
              <li>
                The next step will categorize {categorizedSongCount} songs into{" "}
                {categoryCount} categories.
              </li>
            )}
          </ul>

          {categoriesPreviewError && (
            <p className="scan-wizard-error">{categoriesPreviewError}</p>
          )}
        </div>

        <div className="scan-wizard-actions">
          <button className="primary-btn" type="button" disabled>
            Next
          </button>
        </div>
      </section>
    </div>
  );
}
