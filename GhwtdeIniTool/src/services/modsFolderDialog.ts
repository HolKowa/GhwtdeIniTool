import { open } from "@tauri-apps/plugin-dialog";

export function openModsFolderDialog() {
  return openFolderDialog("Select MODS folder");
}

export function openCategoriesExtraFolderDialog() {
  return openFolderDialog("Select extra category folder");
}

function openFolderDialog(title: string) {
  return open({
    directory: true,
    multiple: false,
    title,
  });
}
