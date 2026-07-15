export type ProjectSettings = {
  disclaimer_accepted: boolean;
  mods_dir: string | null;
  mods_dir_available: boolean;
  official_gamelogos_dir: string | null;
  official_gamelogos_dir_available: boolean;
  keep_original_song_ini: boolean;
  check_for_updates_on_startup: boolean;
  settings_file: string;
};
