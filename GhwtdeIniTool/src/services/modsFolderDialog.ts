import { open } from "@tauri-apps/plugin-dialog";

export function openModsFolderDialog() {
  return openFolderDialog("Select MODS folder");
}

export function openGameLogosFolderDialog() {
  return openFolderDialog("Select official GAMELOGOS folder");
}

export function openBackupImportDialog() {
  return open({ directory: false, multiple: false, title: "Select song backup CSV", filters: [{ name: "CSV", extensions: ["csv"] }] });
}

export function openBackupExportFolderDialog() {
  return openFolderDialog("Select backup export folder");
}

function openFolderDialog(title: string) {
  return open({
    directory: true,
    multiple: false,
    title,
  });
}
