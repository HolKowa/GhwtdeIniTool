import { open } from "@tauri-apps/plugin-dialog";

export function openModsFolderDialog() {
  return openFolderDialog("Select MODS folder");
}

export function openGameLogosFolderDialog() {
  return openFolderDialog("Select official GAMELOGOS folder");
}

function openFolderDialog(title: string) {
  return open({
    directory: true,
    multiple: false,
    title,
  });
}
