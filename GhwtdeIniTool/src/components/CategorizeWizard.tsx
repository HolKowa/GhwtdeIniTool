import { useEffect, useMemo, useState } from "react";

import type { CategorizationSortKey } from "./ScannedSongsTable";
import { previewIniToolCategories } from "../services/scanModsApi";
import type { CategorizeSongsInput, ScannedSong } from "../types/scanMods";

type CategorizeWizardProps = {
  error: string;
  hasActiveFilters: boolean;
  hasUnsavedSongChanges: boolean;
  includedSongCount: number;
  includedSongPaths: string[];
  modsDir: string | null;
  onApply: (input: CategorizeSongsInput) => Promise<void>;
  onClose: () => void;
  orderedSongPaths: string[];
  songs: ScannedSong[];
  sortKey: CategorizationSortKey;
  status: "idle" | "categorizing" | "error";
};

type WizardStep = 1 | 2;
type CategoryPreview = { name: string; checksum: string; folder: string; songCount: number };

const songsPerCategory = 200;
const recommendedSongCap = 4000;
const maximumAllowedSongCap = 19800;
const emptyFieldLabel = "<empty>";

const sortLabels: Record<CategorizationSortKey, string> = {
  artist: "Artist", title: "Title", year: "Year", genre: "Genre", game_icon: "GameIcon",
  guitar: "Guitar", bass: "Bass", drums: "Drums", vocals: "Vocals", coop_guitar: "CoopGuitar", coop_bass: "CoopBass",
};

function displayedSortValue(song: ScannedSong, sortKey: CategorizationSortKey) {
  const value = sortKey in song.instruments
    ? song.instruments[sortKey as keyof ScannedSong["instruments"]].value
    : song[sortKey as keyof Pick<ScannedSong, "artist" | "title" | "year" | "genre" | "game_icon">];
  return value.trim() || emptyFieldLabel;
}

function sanitizeFolderName(value: string) {
  const sanitized = value
    .replace(/[\\/:*?"<>|\x00-\x1F]/g, "")
    .trim()
    .replace(/\s+/g, "_")
    .replace(/[. ]+$/g, "");
  return sanitized || "empty";
}

export function CategorizeWizard(props: CategorizeWizardProps) {
  const { error, hasActiveFilters, hasUnsavedSongChanges, includedSongCount, includedSongPaths, modsDir, onApply, onClose, orderedSongPaths, songs, sortKey, status } = props;
  const [step, setStep] = useState<WizardStep>(1);
  const [maximumSongCap, setMaximumSongCap] = useState(String(recommendedSongCap));
  const [hasNonCategoryFiles, setHasNonCategoryFiles] = useState(false);
  const [categoriesPreviewError, setCategoriesPreviewError] = useState("");
  const parsedMaximumSongCap = Number(maximumSongCap);
  const hasValidMaximumSongCap = Number.isInteger(parsedMaximumSongCap) && parsedMaximumSongCap >= 1 && parsedMaximumSongCap <= maximumAllowedSongCap;
  const categorizedSongCount = hasValidMaximumSongCap ? Math.min(includedSongCount, parsedMaximumSongCap) : 0;
  const excludedForCapCount = hasValidMaximumSongCap ? includedSongCount - categorizedSongCount : 0;
  const categoryFolder = `${modsDir ?? "MODS folder"}/IniToolCategories`;
  const isBusy = status === "categorizing";
  const categoryPreviews = useMemo(() => {
    const songsByPath = new Map(songs.map((song) => [song.relative_path, song]));
    const includedPathSet = new Set(includedSongPaths);
    const categorizedPaths = orderedSongPaths.filter((path) => includedPathSet.has(path)).slice(0, categorizedSongCount);
    const previews: CategoryPreview[] = [];
    for (let start = 0; start < categorizedPaths.length; start += songsPerCategory) {
      const index = previews.length;
      const firstSong = songsByPath.get(categorizedPaths[start]);
      if (!firstSong) continue;
      const value = Array.from(displayedSortValue(firstSong, sortKey)).slice(0, 4).join("");
      const categoryNumber = index + 1;
      const name = `${String(categoryNumber).padStart(2, "0")} ${sortLabels[sortKey]}: ${value}`;
      previews.push({ name, checksum: `IniToolCategory${String(categoryNumber).padStart(2, "0")}`, folder: `Category_${sanitizeFolderName(name)}`, songCount: Math.min(songsPerCategory, categorizedPaths.length - start) });
    }
    return previews;
  }, [categorizedSongCount, includedSongPaths, orderedSongPaths, songs, sortKey]);

  useEffect(() => {
    let isCurrent = true;
    previewIniToolCategories().then((preview) => isCurrent && setHasNonCategoryFiles(preview.has_non_category_files)).catch((err) => isCurrent && setCategoriesPreviewError(String(err)));
    return () => { isCurrent = false; };
  }, []);

  const apply = async () => {
    if (isBusy || !hasValidMaximumSongCap) return;
    try {
      await onApply({ ordered_song_paths: orderedSongPaths, included_song_paths: includedSongPaths, maximum_song_cap: parsedMaximumSongCap, category_names: categoryPreviews.map((category) => category.name) });
      onClose();
    } catch { /* The hook retains the error for display. */ }
  };

  return <div className="scan-wizard-backdrop" role="presentation"><section className="scan-wizard-panel categorize-panel" role="dialog" aria-modal="true" aria-labelledby="categorize-title">
    <div className="scan-wizard-header"><div><p className="scan-wizard-step">Step {step} of 2</p><h2 id="categorize-title">Categorize songs</h2></div><button className="close-btn" type="button" onClick={onClose} aria-label="Close categorize wizard" disabled={isBusy}>&times;</button></div>
    <div className={`scan-wizard-body categorize-wizard-body${step === 2 ? " categorize-step-two-body" : ""}`}>{step === 1 ? <>
      <label className="categorize-cap-field"><span>Cap maximum included songs</span><input aria-invalid={!hasValidMaximumSongCap} min="1" max={maximumAllowedSongCap} step="1" type="number" value={maximumSongCap} onChange={(event) => setMaximumSongCap(event.target.value)} /></label>
      {!hasValidMaximumSongCap && <p className="scan-wizard-error">Enter a whole number from 1 through {maximumAllowedSongCap}.</p>}
      <ul className="categorize-bullet-list"><li className={`categorize-cap-advisory${hasValidMaximumSongCap && parsedMaximumSongCap > recommendedSongCap ? " categorize-warning" : ""}`}>Max 4000 songs are recommended. The game can fail when too many songs are loaded; larger libraries are at your own risk.</li><li>Ordering uses the scanned songs list in the background. Change the table order before opening this wizard.</li>{excludedForCapCount > 0 && <li>The last {excludedForCapCount} songs from the list are excluded to fit within maximum song cap.</li>}<li>Included songs are linked into categories of {songsPerCategory} songs.</li><li className={hasActiveFilters ? "categorize-warning" : undefined}>Filters of the scanned songs list are not used while linking songs.</li><li className={hasUnsavedSongChanges ? "categorize-warning" : undefined}>Unsaved changes of songs are not saved while categorizing the songs.</li><li>All songs not included are renamed to song.excluded.ini so they do not show as uncategorized in the game.</li><li>All included songs get their GameCategory entry edited.</li><li className={hasNonCategoryFiles ? "categorize-warning" : undefined}>All old files within this folder are deleted:<span className="categorize-folder-path">{categoryFolder}</span></li>{hasValidMaximumSongCap && <li>The next step will categorize {categorizedSongCount} songs into {categoryPreviews.length} categories.</li>}</ul>
      {categoriesPreviewError && <p className="scan-wizard-error">{categoriesPreviewError}</p>}
    </> : <>
      <p className="scan-wizard-summary">Review the categories that will be created.</p>
      <div className="categorize-preview-list">{categoryPreviews.map((category) => <section className="categorize-preview" key={category.checksum}>
        <div><strong>CategoryName:</strong><span>{category.name}</span></div>
        <div><strong>Songs:</strong><span>{category.songCount}</span></div>
        <div><strong>CategoryChecksum:</strong><span>{category.checksum}</span></div>
        <div><strong>Folder:</strong><span>{categoryFolder}/{category.folder}</span></div>
      </section>)}</div>
      {status === "error" && <p className="scan-wizard-error">{error}</p>}{isBusy && <p className="scan-wizard-summary">Categorizing songs...</p>}
    </>}</div>
    <div className="scan-wizard-actions">{step === 1 ? <button className="primary-btn" type="button" onClick={() => setStep(2)} disabled={!hasValidMaximumSongCap}>Next</button> : <><button className="secondary-btn" type="button" onClick={() => setStep(1)} disabled={isBusy}>Back</button><button className="primary-btn" type="button" onClick={apply} disabled={isBusy}>{isBusy ? "Categorizing..." : "Categorize songs"}</button></>}</div>
  </section></div>;
}
