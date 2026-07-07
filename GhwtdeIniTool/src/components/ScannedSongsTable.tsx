import { useEffect, useMemo, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";

import type {
  DuplicateChecksumGroup,
  ScannedSong,
  ScannedSongMetadata,
} from "../types/scanMods";

type ScannedSongsTableProps = {
  duplicateChecksumGroups: DuplicateChecksumGroup[];
  includedSongPaths: string[];
  onCopyFolderPath: (absolutePath: string) => void;
  onRestoreOriginal: (relativePath: string) => Promise<void>;
  onSaveMetadata: (
    relativePath: string,
    metadata: ScannedSongMetadata,
  ) => Promise<void>;
  onSetSongIncluded: (relativePath: string, isIncluded: boolean) => void;
  onSetSongsIncluded: (relativePaths: string[], isIncluded: boolean) => void;
  restoringSongPaths: string[];
  savingSongPaths: string[];
  songs: ScannedSong[];
};

type MetadataColumnKey = "artist" | "title" | "year" | "genre" | "game_icon";
type InstrumentColumnKey = keyof ScannedSong["instruments"];
type SortKey = MetadataColumnKey | InstrumentColumnKey;
type SortDirection = "asc" | "desc";
type IncludeFilter = "all" | "included" | "excluded";
type MetadataDropdownFilterKey = "year" | "genre" | "game_icon";
type ContextMenuState = {
  x: number;
  y: number;
};
type Column =
  | { key: MetadataColumnKey; label: string; type: "metadata" }
  | { key: InstrumentColumnKey; label: string; type: "instrument" };

const emptyFieldLabel = "<empty>";
const metadataDropdownFilterKeys = new Set<MetadataColumnKey>([
  "year",
  "genre",
  "game_icon",
]);

const columns: Column[] = [
  { key: "artist", label: "Artist", type: "metadata" },
  { key: "title", label: "Title", type: "metadata" },
  { key: "year", label: "Year", type: "metadata" },
  { key: "genre", label: "Genre", type: "metadata" },
  { key: "game_icon", label: "GameIcon", type: "metadata" },
  { key: "guitar", label: "Guitar", type: "instrument" },
  { key: "bass", label: "Bass", type: "instrument" },
  { key: "drums", label: "Drums", type: "instrument" },
  { key: "vocals", label: "Vocals", type: "instrument" },
  { key: "coop_guitar", label: "CoopGuitar", type: "instrument" },
  { key: "coop_bass", label: "CoopBass", type: "instrument" },
];

const defaultColumnWidths: Record<SortKey, number> = {
  artist: 240,
  title: 315,
  year: 100,
  genre: 150,
  game_icon: 138,
  guitar: 90,
  bass: 90,
  drums: 90,
  vocals: 90,
  coop_guitar: 104,
  coop_bass: 98,
};
const minColumnWidth = 70;
const includeColumnWidth = 104;
const actionsColumnWidth = 184;

const emptyFilters: Record<SortKey, string> = {
  artist: "",
  title: "",
  year: "",
  genre: "",
  game_icon: "",
  guitar: "",
  bass: "",
  drums: "",
  vocals: "",
  coop_guitar: "",
  coop_bass: "",
};

const instrumentValueRanks: Record<string, number> = {
  Error: 0,
  Unknown: 1,
  No: 2,
  Easy: 3,
  Medium: 4,
  Hard: 5,
  Expert: 6,
};

export function ScannedSongsTable({
  duplicateChecksumGroups,
  includedSongPaths,
  onCopyFolderPath,
  onRestoreOriginal,
  onSaveMetadata,
  onSetSongIncluded,
  onSetSongsIncluded,
  restoringSongPaths,
  savingSongPaths,
  songs,
}: ScannedSongsTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>("artist");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [includeFilter, setIncludeFilter] = useState<IncludeFilter>("all");
  const [filters, setFilters] = useState<Record<SortKey, string>>(emptyFilters);
  const [columnWidths, setColumnWidths] =
    useState<Record<SortKey, number>>(defaultColumnWidths);
  const [editedRows, setEditedRows] = useState<
    Record<string, ScannedSongMetadata>
  >({});
  const [selectedSongPaths, setSelectedSongPaths] = useState<Set<string>>(
    () => new Set(),
  );
  const [lastSelectedSongPath, setLastSelectedSongPath] = useState("");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const includedSongPathSet = useMemo(
    () => new Set(includedSongPaths),
    [includedSongPaths],
  );
  const duplicateSongPathSet = useMemo(() => {
    const pathSet = new Set<string>();

    for (const group of duplicateChecksumGroups) {
      for (const relativePath of group.relative_paths) {
        pathSet.add(relativePath);
      }
    }

    return pathSet;
  }, [duplicateChecksumGroups]);
  const savingSongPathSet = useMemo(
    () => new Set(savingSongPaths),
    [savingSongPaths],
  );
  const restoringSongPathSet = useMemo(
    () => new Set(restoringSongPaths),
    [restoringSongPaths],
  );
  const tableWidth =
    columns.reduce((total, column) => total + columnWidths[column.key], 0) +
    includeColumnWidth +
    actionsColumnWidth;
  const metadataDropdownFilterOptions = useMemo(() => {
    const options = {} as Record<MetadataDropdownFilterKey, string[]>;

    metadataDropdownFilterKeys.forEach((key) => {
      const valueSet = new Set(songs.map((song) => displayValue(song[key])));

      options[key as MetadataDropdownFilterKey] = Array.from(valueSet).sort(
        (left, right) =>
          left.localeCompare(right, undefined, {
            numeric: true,
            sensitivity: "base",
          }),
      );
    });

    return options;
  }, [songs]);
  const instrumentFilterOptions = useMemo(() => {
    const options = {} as Record<InstrumentColumnKey, string[]>;

    columns.forEach((column) => {
      if (column.type !== "instrument") {
        return;
      }

      const valueSet = new Set(
        songs.map((song) => song.instruments[column.key].value),
      );

      options[column.key] = Array.from(valueSet).sort((left, right) => {
        const rankComparison =
          (instrumentValueRanks[left] ?? 0) - (instrumentValueRanks[right] ?? 0);

        if (rankComparison !== 0) {
          return rankComparison;
        }

        return left.localeCompare(right, undefined, {
          numeric: true,
          sensitivity: "base",
        });
      });
    });

    return options;
  }, [songs]);

  const filteredSongs = useMemo(() => {
    const activeFilters = columns
      .map((column) => ({
        column,
        key: column.key,
        value: filters[column.key].trim().toLocaleLowerCase(),
      }))
      .filter((filter) => filter.value.length > 0);

    const nextSongs = songs.filter((song) => {
      const isIncluded = includedSongPathSet.has(song.relative_path);

      if (includeFilter === "included" && !isIncluded) {
        return false;
      }

      if (includeFilter === "excluded" && isIncluded) {
        return false;
      }

      return activeFilters.every((filter) =>
        matchesFilter(song, filter.column, filter.value),
      );
    });

    nextSongs.sort((left, right) => {
      const direction = sortDirection === "asc" ? 1 : -1;
      const comparison = compareColumnValues(left, right, sortKey);

      if (comparison !== 0) {
        return comparison * direction;
      }

      return left.relative_path.localeCompare(right.relative_path) * direction;
    });

    return nextSongs;
  }, [filters, includeFilter, includedSongPathSet, songs, sortDirection, sortKey]);
  const filteredSongPathSet = useMemo(
    () => new Set(filteredSongs.map((song) => song.relative_path)),
    [filteredSongs],
  );
  const includedSongCount = useMemo(
    () =>
      songs.reduce(
        (total, song) =>
          includedSongPathSet.has(song.relative_path) ? total + 1 : total,
        0,
      ),
    [includedSongPathSet, songs],
  );

  useEffect(() => {
    const currentSongPathSet = new Set(songs.map((song) => song.relative_path));

    setSelectedSongPaths((currentSelectedSongPaths) => {
      const nextSelectedSongPaths = new Set<string>();

      currentSelectedSongPaths.forEach((path) => {
        if (currentSongPathSet.has(path) && filteredSongPathSet.has(path)) {
          nextSelectedSongPaths.add(path);
        }
      });

      return nextSelectedSongPaths;
    });

    setLastSelectedSongPath((currentLastSelectedSongPath) =>
      currentSongPathSet.has(currentLastSelectedSongPath) &&
      filteredSongPathSet.has(currentLastSelectedSongPath)
        ? currentLastSelectedSongPath
        : "",
    );
  }, [filteredSongPathSet, songs]);

  useEffect(() => {
    if (!contextMenu) {
      return;
    }

    function closeContextMenu() {
      setContextMenu(null);
    }

    function closeContextMenuOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        closeContextMenu();
      }
    }

    document.addEventListener("click", closeContextMenu);
    document.addEventListener("keydown", closeContextMenuOnEscape);
    window.addEventListener("scroll", closeContextMenu, true);

    return () => {
      document.removeEventListener("click", closeContextMenu);
      document.removeEventListener("keydown", closeContextMenuOnEscape);
      window.removeEventListener("scroll", closeContextMenu, true);
    };
  }, [contextMenu]);

  function updateFilter(key: SortKey, value: string) {
    setFilters((currentFilters) => ({
      ...currentFilters,
      [key]: value,
    }));
    setContextMenu(null);
  }

  function updateIncludeFilter(value: IncludeFilter) {
    setIncludeFilter(value);
    setContextMenu(null);
  }

  function songMetadata(song: ScannedSong): ScannedSongMetadata {
    return {
      artist: song.artist,
      title: song.title,
      year: song.year,
      genre: song.genre,
      game_icon: song.game_icon,
    };
  }

  function rowMetadata(song: ScannedSong) {
    return editedRows[song.relative_path] ?? songMetadata(song);
  }

  function isRowDirty(song: ScannedSong) {
    const metadata = editedRows[song.relative_path];

    if (!metadata) {
      return false;
    }

    return columns.some(
      (column) =>
        column.type === "metadata" && metadata[column.key] !== song[column.key],
    );
  }

  function updateMetadataValue(
    song: ScannedSong,
    key: MetadataColumnKey,
    value: string,
  ) {
    setEditedRows((currentRows) => {
      const currentMetadata = currentRows[song.relative_path] ?? songMetadata(song);
      const nextMetadata = {
        ...currentMetadata,
        [key]: value,
      };
      const isDirty = columns.some(
        (column) =>
          column.type === "metadata" &&
          nextMetadata[column.key] !== song[column.key],
      );

      if (!isDirty) {
        const { [song.relative_path]: _removed, ...remainingRows } =
          currentRows;

        return remainingRows;
      }

      return {
        ...currentRows,
        [song.relative_path]: nextMetadata,
      };
    });
  }

  async function saveRow(song: ScannedSong) {
    try {
      await onSaveMetadata(song.relative_path, rowMetadata(song));
      setEditedRows((currentRows) => {
        const { [song.relative_path]: _removed, ...remainingRows } =
          currentRows;

        return remainingRows;
      });
    } catch {
      // Toast is handled by the scanner hook; keep edits so the user can retry.
    }
  }

  async function restoreRow(song: ScannedSong) {
    try {
      await onRestoreOriginal(song.relative_path);
      setEditedRows((currentRows) => {
        const { [song.relative_path]: _removed, ...remainingRows } =
          currentRows;

        return remainingRows;
      });
    } catch {
      // Toast is handled by the scanner hook; keep edits so the user can retry.
    }
  }

  function changeSort(nextKey: SortKey) {
    if (sortKey === nextKey) {
      setSortDirection((currentDirection) =>
        currentDirection === "asc" ? "desc" : "asc",
      );
      setContextMenu(null);
      return;
    }

    setSortKey(nextKey);
    setSortDirection("asc");
    setContextMenu(null);
  }

  function resizeColumn(columnIndex: number, event: ReactMouseEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const leftKey = columns[columnIndex].key;
    const rightKey = columns[columnIndex + 1].key;
    const startLeftWidth = columnWidths[leftKey];
    const startRightWidth = columnWidths[rightKey];

    function onMouseMove(moveEvent: MouseEvent) {
      const requestedDelta = moveEvent.clientX - startX;
      const minDelta = minColumnWidth - startLeftWidth;
      const maxDelta = startRightWidth - minColumnWidth;
      const delta = Math.min(maxDelta, Math.max(minDelta, requestedDelta));

      setColumnWidths((currentWidths) => ({
        ...currentWidths,
        [leftKey]: startLeftWidth + delta,
        [rightKey]: startRightWidth - delta,
      }));
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  function displayValue(value: string) {
    const trimmedValue = value.trim();

    return trimmedValue.length > 0 ? value : emptyFieldLabel;
  }

  function displayCell(value: string) {
    const displayedValue = displayValue(value);
    const isEmpty = displayedValue === emptyFieldLabel;

    return (
      <span className={isEmpty ? "scanned-songs-empty-value" : undefined}>
        {displayedValue}
      </span>
    );
  }

  function metadataCell(song: ScannedSong, column: MetadataColumnKey) {
    return (
      <input
        aria-label={`${column} for ${song.relative_path}`}
        className="scanned-songs-metadata-input"
        value={rowMetadata(song)[column]}
        onChange={(event) =>
          updateMetadataValue(song, column, event.target.value)
        }
        onClick={(event) => event.stopPropagation()}
        onContextMenu={(event) => event.stopPropagation()}
      />
    );
  }

  function toggleIncludedSong(song: ScannedSong) {
    onSetSongIncluded(
      song.relative_path,
      !includedSongPathSet.has(song.relative_path),
    );
  }

  function selectSong(song: ScannedSong, event: ReactMouseEvent) {
    const isRangeSelection = event.shiftKey && lastSelectedSongPath.length > 0;
    const isToggleSelection = event.ctrlKey || event.metaKey;

    if (isRangeSelection) {
      const visibleSongPaths = filteredSongs.map(
        (filteredSong) => filteredSong.relative_path,
      );
      const anchorIndex = visibleSongPaths.indexOf(lastSelectedSongPath);
      const songIndex = visibleSongPaths.indexOf(song.relative_path);

      if (anchorIndex >= 0 && songIndex >= 0) {
        const startIndex = Math.min(anchorIndex, songIndex);
        const endIndex = Math.max(anchorIndex, songIndex);

        setSelectedSongPaths(
          new Set(visibleSongPaths.slice(startIndex, endIndex + 1)),
        );
        return;
      }
    }

    setSelectedSongPaths((currentSelectedSongPaths) => {
      if (isToggleSelection) {
        const nextSelectedSongPaths = new Set(currentSelectedSongPaths);

        if (nextSelectedSongPaths.has(song.relative_path)) {
          nextSelectedSongPaths.delete(song.relative_path);
        } else {
          nextSelectedSongPaths.add(song.relative_path);
        }

        return nextSelectedSongPaths;
      }

      return new Set([song.relative_path]);
    });
    setLastSelectedSongPath(song.relative_path);
    setContextMenu(null);
  }

  function preventNativeModifierSelection(event: ReactMouseEvent) {
    if (!event.shiftKey && !event.ctrlKey && !event.metaKey) {
      return;
    }

    event.preventDefault();
    window.getSelection()?.removeAllRanges();
  }

  function openSelectionMenu(song: ScannedSong, event: ReactMouseEvent) {
    event.preventDefault();

    if (!selectedSongPaths.has(song.relative_path)) {
      setSelectedSongPaths(new Set([song.relative_path]));
      setLastSelectedSongPath(song.relative_path);
    }

    setContextMenu({
      x: event.clientX,
      y: event.clientY,
    });
  }

  function updateSelectedIncludedState(isIncluded: boolean) {
    onSetSongsIncluded(
      Array.from(selectedSongPaths).filter((path) =>
        filteredSongPathSet.has(path),
      ),
      isIncluded,
    );
    setContextMenu(null);
  }

  function columnValue(song: ScannedSong, column: Column) {
    if (column.type === "instrument") {
      return song.instruments[column.key].value;
    }

    return displayValue(song[column.key]);
  }

  function columnTitle(song: ScannedSong, column: Column) {
    if (column.type === "instrument") {
      return song.instruments[column.key].tooltip;
    }

    return song[column.key];
  }

  function matchesFilter(song: ScannedSong, column: Column, filter: string) {
    if (column.type === "instrument") {
      const value = song.instruments[column.key].value.toLocaleLowerCase();

      return value === filter;
    }

    if (metadataDropdownFilterKeys.has(column.key)) {
      return displayValue(song[column.key]).toLocaleLowerCase() === filter;
    }

    return displayValue(song[column.key]).toLocaleLowerCase().includes(filter);
  }

  function compareColumnValues(left: ScannedSong, right: ScannedSong, key: SortKey) {
    const column = columns.find((candidate) => candidate.key === key);

    if (column?.type === "instrument") {
      const leftRank =
        instrumentValueRanks[left.instruments[key as InstrumentColumnKey].value] ?? 0;
      const rightRank =
        instrumentValueRanks[right.instruments[key as InstrumentColumnKey].value] ?? 0;

      return leftRank - rightRank;
    }

    const leftValue = left[key as MetadataColumnKey].toLocaleLowerCase();
    const rightValue = right[key as MetadataColumnKey].toLocaleLowerCase();

    return leftValue.localeCompare(rightValue, undefined, {
      numeric: true,
      sensitivity: "base",
    });
  }

  return (
    <section className="scanned-songs" aria-labelledby="scanned-songs-title">
      <div className="scanned-songs-header">
        <h1 id="scanned-songs-title">Scanned songs</h1>
        <span>
          included: {includedSongCount} - visible: {filteredSongs.length} -
          total: {songs.length}
        </span>
      </div>

      {songs.length === 0 ? (
        <p className="scanned-songs-empty">No parsed songs found.</p>
      ) : (
        <div className="scanned-songs-table-wrap">
          <table
            className="scanned-songs-table"
            style={{ minWidth: tableWidth }}
          >
            <colgroup>
              <col style={{ width: includeColumnWidth }} />
              {columns.map((column) => (
                <col
                  key={column.key}
                  style={{ width: columnWidths[column.key] }}
                />
              ))}
              <col style={{ width: actionsColumnWidth }} />
            </colgroup>
            <thead>
              <tr>
                <th className="scanned-songs-include-column">Include</th>
                {columns.map((column, index) => (
                  <th className="scanned-songs-resizable" key={column.key}>
                    <button type="button" onClick={() => changeSort(column.key)}>
                      {column.label}
                      <span aria-hidden="true">
                        {sortKey === column.key
                          ? sortDirection === "asc"
                            ? "asc"
                            : "desc"
                          : ""}
                      </span>
                    </button>
                    {index < columns.length - 1 && (
                      <span
                        aria-label={`Resize ${column.label} column`}
                        className="scanned-songs-resize-handle"
                        onMouseDown={(event) => resizeColumn(index, event)}
                        role="separator"
                      />
                    )}
                  </th>
                ))}
                <th className="scanned-songs-action-column">
                  {"Actions"}
                </th>
              </tr>
              <tr className="scanned-songs-filters">
                <th className="scanned-songs-include-column">
                  <select
                    aria-label="Filter Include"
                    value={includeFilter}
                    onChange={(event) =>
                      updateIncludeFilter(event.target.value as IncludeFilter)
                    }
                  >
                    <option value="all">All</option>
                    <option value="included">Included</option>
                    <option value="excluded">Excluded</option>
                  </select>
                </th>
                {columns.map((column) => (
                  <th key={column.key}>
                    {column.type === "metadata" &&
                    metadataDropdownFilterKeys.has(column.key) ? (
                      <select
                        aria-label={`Filter ${column.label}`}
                        value={filters[column.key]}
                        onChange={(event) =>
                          updateFilter(column.key, event.target.value)
                        }
                      >
                        <option value="">All</option>
                        {metadataDropdownFilterOptions[
                          column.key as MetadataDropdownFilterKey
                        ].map((value) => (
                          <option key={value} value={value}>
                            {value}
                          </option>
                        ))}
                      </select>
                    ) : column.type === "instrument" ? (
                      <select
                        aria-label={`Filter ${column.label}`}
                        value={filters[column.key]}
                        onChange={(event) =>
                          updateFilter(column.key, event.target.value)
                        }
                      >
                        <option value="">All</option>
                        {instrumentFilterOptions[column.key].map((value) => (
                          <option key={value} value={value}>
                            {value}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        aria-label={`Filter ${column.label}`}
                        value={filters[column.key]}
                        onChange={(event) =>
                          updateFilter(column.key, event.target.value)
                        }
                      />
                    )}
                  </th>
                ))}
                <th className="scanned-songs-action-column" />
              </tr>
            </thead>
            <tbody>
              {filteredSongs.map((song) => {
                const isSaving = savingSongPathSet.has(song.relative_path);
                const isRestoring = restoringSongPathSet.has(song.relative_path);
                const isRowBusy = isSaving || isRestoring;
                const isDirty = isRowDirty(song);
                const isDuplicate = duplicateSongPathSet.has(song.relative_path);
                const isIncluded = includedSongPathSet.has(song.relative_path);
                const isSelected = selectedSongPaths.has(song.relative_path);
                const rowClassName = [
                  isSelected ? "selected" : "",
                  isDuplicate ? "duplicate-checksum" : "",
                ]
                  .filter(Boolean)
                  .join(" ");

                return (
                  <tr
                    aria-selected={isSelected}
                    className={rowClassName || undefined}
                    key={song.relative_path}
                    onClick={(event) => selectSong(song, event)}
                    onContextMenu={(event) => openSelectionMenu(song, event)}
                    onMouseDownCapture={preventNativeModifierSelection}
                  >
                    <td className="scanned-songs-include-column">
                      <input
                        aria-label={`Include ${song.relative_path}`}
                        checked={isIncluded}
                        type="checkbox"
                        onChange={() => toggleIncludedSong(song)}
                        onClick={(event) => event.stopPropagation()}
                        onContextMenu={(event) => event.stopPropagation()}
                      />
                    </td>
                    {columns.map((column) => (
                      <td key={column.key} title={columnTitle(song, column)}>
                        {column.type === "metadata"
                          ? metadataCell(song, column.key)
                          : displayCell(columnValue(song, column))}
                      </td>
                    ))}
                    <td className="scanned-songs-action-column">
                      <div
                        className="scanned-songs-actions"
                        onClick={(event) => event.stopPropagation()}
                        onContextMenu={(event) => event.stopPropagation()}
                      >
                        <button
                          className="scanned-songs-action-btn"
                          type="button"
                          onClick={() =>
                            onCopyFolderPath(song.folder_absolute_path)
                          }
                        >
                          Folder
                        </button>
                        <button
                          className="scanned-songs-action-btn"
                          type="button"
                          onClick={() => saveRow(song)}
                          disabled={!isDirty || isRowBusy}
                        >
                          {isSaving ? "Saving" : "Save"}
                        </button>
                        <button
                          className="scanned-songs-action-btn"
                          type="button"
                          onClick={() => restoreRow(song)}
                          disabled={!song.has_original_song_ini || isRowBusy}
                        >
                          {isRestoring ? "Restoring" : "Restore"}
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          {contextMenu && (
            <div
              className="scanned-songs-context-menu"
              style={{
                left: contextMenu.x,
                top: contextMenu.y,
              }}
              onClick={(event) => event.stopPropagation()}
              role="menu"
            >
              <button
                type="button"
                onClick={() => updateSelectedIncludedState(true)}
                role="menuitem"
              >
                Include selection
              </button>
              <button
                type="button"
                onClick={() => updateSelectedIncludedState(false)}
                role="menuitem"
              >
                Exclude selection
              </button>
            </div>
          )}

          {filteredSongs.length === 0 && (
            <p className="scanned-songs-empty">No songs match the filters.</p>
          )}
        </div>
      )}
    </section>
  );
}
