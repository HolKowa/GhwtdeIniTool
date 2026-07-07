import { useMemo, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";

import type { ScannedSong, ScannedSongMetadata } from "../types/scanMods";

type ScannedSongsTableProps = {
  onCopyFolderPath: (absolutePath: string) => void;
  onRestoreOriginal: (relativePath: string) => Promise<void>;
  onSaveMetadata: (
    relativePath: string,
    metadata: ScannedSongMetadata,
  ) => Promise<void>;
  restoringSongPaths: string[];
  savingSongPaths: string[];
  songs: ScannedSong[];
};

type MetadataColumnKey = "artist" | "title" | "year" | "genre" | "game_icon";
type InstrumentColumnKey = keyof ScannedSong["instruments"];
type SortKey = MetadataColumnKey | InstrumentColumnKey;
type SortDirection = "asc" | "desc";
type Column =
  | { key: MetadataColumnKey; label: string; type: "metadata" }
  | { key: InstrumentColumnKey; label: string; type: "instrument" };

const emptyFieldLabel = "<empty>";

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
  artist: 255,
  title: 330,
  year: 110,
  genre: 166,
  game_icon: 150,
  guitar: 96,
  bass: 96,
  drums: 96,
  vocals: 96,
  coop_guitar: 110,
  coop_bass: 103,
};
const minColumnWidth = 70;
const actionsColumnWidth = 220;

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
  onCopyFolderPath,
  onRestoreOriginal,
  onSaveMetadata,
  restoringSongPaths,
  savingSongPaths,
  songs,
}: ScannedSongsTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>("artist");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [filters, setFilters] = useState<Record<SortKey, string>>(emptyFilters);
  const [columnWidths, setColumnWidths] =
    useState<Record<SortKey, number>>(defaultColumnWidths);
  const [editedRows, setEditedRows] = useState<
    Record<string, ScannedSongMetadata>
  >({});
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
    actionsColumnWidth;

  const filteredSongs = useMemo(() => {
    const activeFilters = columns
      .map((column) => ({
        column,
        key: column.key,
        value: filters[column.key].trim().toLocaleLowerCase(),
      }))
      .filter((filter) => filter.value.length > 0);

    const nextSongs = songs.filter((song) =>
      activeFilters.every((filter) =>
        matchesFilter(song, filter.column, filter.value),
      ),
    );

    nextSongs.sort((left, right) => {
      const direction = sortDirection === "asc" ? 1 : -1;
      const comparison = compareColumnValues(left, right, sortKey);

      if (comparison !== 0) {
        return comparison * direction;
      }

      return left.relative_path.localeCompare(right.relative_path) * direction;
    });

    return nextSongs;
  }, [filters, songs, sortDirection, sortKey]);

  function updateFilter(key: SortKey, value: string) {
    setFilters((currentFilters) => ({
      ...currentFilters,
      [key]: value,
    }));
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
      return;
    }

    setSortKey(nextKey);
    setSortDirection("asc");
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
      />
    );
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

      return value.includes(filter);
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
          {filteredSongs.length} of {songs.length}
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
                {columns.map((column) => (
                  <th key={column.key}>
                    <input
                      aria-label={`Filter ${column.label}`}
                      value={filters[column.key]}
                      onChange={(event) =>
                        updateFilter(column.key, event.target.value)
                      }
                    />
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

                return (
                  <tr key={song.relative_path}>
                    {columns.map((column) => (
                      <td key={column.key} title={columnTitle(song, column)}>
                        {column.type === "metadata"
                          ? metadataCell(song, column.key)
                          : displayCell(columnValue(song, column))}
                      </td>
                    ))}
                    <td className="scanned-songs-action-column">
                      <div className="scanned-songs-actions">
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

          {filteredSongs.length === 0 && (
            <p className="scanned-songs-empty">No songs match the filters.</p>
          )}
        </div>
      )}
    </section>
  );
}
