export type ScanModsPreview = {
  categories_found: number;
  files_to_move: string[];
  errors: string[];
  moved_categories_enabled: boolean;
};

export type ScanModsResult = {
  categories_found: number;
  category_folders_moved: number;
  files_moved: number;
  renamed_destinations: number;
  errors: string[];
  moved_categories_enabled: boolean;
};

export type DeleteFilesPreview = {
  files_to_delete: string[];
  errors: string[];
};

export type DeleteFilesResult = {
  files_deleted: number;
  errors: string[];
};

export type FaultySongIniFile = {
  relative_path: string;
  contents: string;
  error: string;
};

export type SongIniScanResult = {
  songs_found: number;
  songs_parsed: number;
  faulty_files: FaultySongIniFile[];
  errors: string[];
};

export type SongIniValidationResult = {
  relative_path: string;
  contents: string;
  songs_parsed: number;
};
