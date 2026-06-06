import { open } from "@tauri-apps/plugin-dialog";

export function openModsFolderDialog() {
  return open({
    directory: true,
    multiple: false,
    title: "Select MODS folder",
  });
}
