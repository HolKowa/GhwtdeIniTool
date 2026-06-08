export type ScanModsResult = {
  categories_found: number;
  category_folders_moved: number;
  files_moved: number;
  renamed_destinations: number;
  errors: string[];
  moved_categories_enabled: boolean;
};
