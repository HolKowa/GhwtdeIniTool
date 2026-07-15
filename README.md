# GhwtdeIniTool

GhwtdeIniTool is a desktop utility for managing and repairing Guitar Hero
World Tour Definitive Edition MODS content.

## Overview

- Scan and repair `song.ini` files in a MODS folder.
- Restore backups, clean unwanted MODS files, and analyze instrument charts.
- Fix GameIcons and create game-ready song categories.

## Before you start

GhwtdeIniTool is experimental software that modifies files in the selected
MODS folder. It can overwrite, rename, delete, or create files. Back up your
game files and MODS folder before using it.

The application needs both of these folders in **Settings** before workflows
are available:

- Your GHWTDE `MODS` folder.
- The official `GAMELOGOS` folder, used to validate GameIcons.

Leave **Keep original on first song.ini change** enabled unless you have a
specific reason not to. It preserves an original backup before the tool first
edits a song INI file.

## Download

Download the latest release from the
[GitHub Releases page](https://github.com/HolKowa/GhwtdeIniLoader/releases).

## Quick start

1. Download and launch the latest release.
2. Open **Settings**, select your MODS folder and official GAMELOGOS folder,
   then accept the experimental-software warning.
3. Keep **Keep original on first song.ini change** enabled.
4. Select **Scan MODS folder** and resolve the format, duplicate, conflicting
   INI, and missing-content issues shown by the wizard.
5. Review the scanned-song table, edit and save song metadata, and choose and
   sort the songs to include.
6. Select **Categorize**, review the proposed categories, and apply them to
   create the game-ready category folders and logos.

## Workflows

### Scan and repair `song.ini` files

Select **Scan MODS folder** to check active and excluded song INI files. Use
the wizard to repair invalid entries, resolve duplicate checksums and multiple
INI variants in a folder, and address missing content files. Repairs can edit
or rename files; disabling a song renames its active INI so the game will not
load it. Complete a scan before using **Analyze instruments**, **Fix
GameIcons**, or **Categorize**.

### Restore INI files

Select **Restore INI files** to restore `song.original.ini` backups, restore
the state before format fixes when applicable, or delete generated
`song.instruments.ini` sidecars. Restore actions overwrite active INIs or
delete sidecar files, so use them only when you intend to discard those
changes.

### Clean the MODS folder

Select **Clean MODS folder**, enter or review the keep pattern, and inspect
the deletion preview. Only the selected previewed files are deleted. This is a
destructive cleanup action, so confirm the list carefully.

### Analyze instruments

After a completed scan, select **Analyze instruments** to read chart data and
show available instrument difficulties in the scanned-song table. The analysis
creates or updates `song.instruments.ini` sidecar files next to songs.

### Fix GameIcons

After a completed scan, select **Fix GameIcons** to check custom
`gamelogo_*.img.xen` files and their `category.ini` metadata, then correct
invalid song GameIcon values. Fixing can move custom logo files, replace
category metadata, and edit `song.ini` files.

### Categorize songs

After a completed scan, sort the table and mark the songs you want to
categorize as included. Select one or multiple songs, then right-click the
selection to include or exclude them together. Songs not marked as included
are excluded when categorization runs, so make sure the intended songs are
included before selecting **Categorize**. Review the generated categories
before applying them. This workflow updates included songs, excludes remaining
songs, and recreates the `IniToolCategories` output folder with category
metadata and logos.

## Recommended workflow

Back up your files → configure Settings → scan and repair → review the song
table → optionally analyze instruments or fix GameIcons → categorize songs.

## Development

Developer setup, building, updater testing, signing, and release instructions
are available in the [development guide](docs/DEVELOPMENT.md).

## License

This project is licensed under the [MIT License](LICENSE).

Third-party libraries, fonts, and other components remain subject to their
respective licenses. See
[THIRD_PARTY_LICENSES.txt](GhwtdeIniTool/src-tauri/resources/THIRD_PARTY_LICENSES.txt)
for details.
