import { useMemo, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";

import type { ScannedSong } from "../types/scanMods";

type ScannedSongsTableProps = {
  onCopyFolderPath: (absolutePath: string) => void;
  songs: ScannedSong[];
};

type SortKey = "artist" | "title" | "year" | "genre" | "game_icon";
type SortDirection = "asc" | "desc";

const emptyFieldLabel = "<empty>";

const columns: Array<{ key: SortKey; label: string }> = [
  { key: "artist", label: "Artist" },
  { key: "title", label: "Title" },
  { key: "year", label: "Year" },
  { key: "genre", label: "Genre" },
  { key: "game_icon", label: "GameIcon" },
];

const defaultColumnWidths: Record<SortKey, number> = {
  artist: 280,
  title: 360,
  year: 110,
  genre: 180,
  game_icon: 166,
};
const minColumnWidth = 70;
const folderColumnWidth = 96;

const emptyFilters: Record<SortKey, string> = {
  artist: "",
  title: "",
  year: "",
  genre: "",
  game_icon: "",
};

export function ScannedSongsTable({
  onCopyFolderPath,
  songs,
}: ScannedSongsTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>("artist");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [filters, setFilters] = useState<Record<SortKey, string>>(emptyFilters);
  const [columnWidths, setColumnWidths] =
    useState<Record<SortKey, number>>(defaultColumnWidths);
  const tableWidth =
    columns.reduce((total, column) => total + columnWidths[column.key], 0) +
    folderColumnWidth;

  const filteredSongs = useMemo(() => {
    const activeFilters = columns
      .map((column) => ({
        key: column.key,
        value: filters[column.key].trim().toLocaleLowerCase(),
      }))
      .filter((filter) => filter.value.length > 0);

    const nextSongs = songs.filter((song) =>
      activeFilters.every((filter) =>
        displayValue(song[filter.key])
          .toLocaleLowerCase()
          .includes(filter.value),
      ),
    );

    nextSongs.sort((left, right) => {
      const direction = sortDirection === "asc" ? 1 : -1;
      const leftValue = left[sortKey].toLocaleLowerCase();
      const rightValue = right[sortKey].toLocaleLowerCase();
      const comparison = leftValue.localeCompare(rightValue, undefined, {
        numeric: true,
        sensitivity: "base",
      });

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
              <col style={{ width: folderColumnWidth }} />
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
                <th className="scanned-songs-folder-column">
                  {"Folder"}
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
                <th className="scanned-songs-folder-column" />
              </tr>
            </thead>
            <tbody>
              {filteredSongs.map((song) => (
                <tr key={song.relative_path}>
                  <td title={song.artist}>{displayCell(song.artist)}</td>
                  <td title={song.title}>{displayCell(song.title)}</td>
                  <td title={song.year}>{displayCell(song.year)}</td>
                  <td title={song.genre}>{displayCell(song.genre)}</td>
                  <td title={song.game_icon}>{displayCell(song.game_icon)}</td>
                  <td className="scanned-songs-folder-column">
                    <button
                      className="scanned-songs-folder-btn"
                      type="button"
                      onClick={() => onCopyFolderPath(song.folder_absolute_path)}
                    >
                      Copy
                    </button>
                  </td>
                </tr>
              ))}
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
