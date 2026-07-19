export type DeleteFilesPreview = {
  files_to_delete: string[];
  errors: string[];
};

export type DeleteFilesResult = {
  files_deleted: number;
  errors: string[];
};

export type IniToolCategoriesPreview = {
  has_non_category_files: boolean;
};

export type CategorizeSongsInput = {
  ordered_song_paths: string[];
  included_song_paths: string[];
  maximum_song_cap: number;
  category_names: string[];
};

export type CategorizeSongsResult = {
  categorized: number;
  excluded: number;
  songs_parsed: number;
  songs: ScannedSong[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  song_ini_folder_conflicts: SongIniFolderConflict[];
  content_file_issues: SongContentIssue[];
};

export type RestoreIniAction =
  | "afterFormatIssueFixes"
  | "beforeFormatIssueFix"
  | "deleteInstrumentSidecars";

export type RestoreIniResult = {
  files_restored: number;
  files_deleted: number;
  errors: string[];
};

export type FaultySongIniFile = {
  relative_path: string;
  contents: string;
  error: string;
};

export type DuplicateChecksumGroup = {
  checksum: string;
  relative_paths: string[];
};

export type SongIniFolderConflict = {
  folder_path: string;
  file_paths: string[];
};

export type SongContentIssue = {
  song_ini_relative_path: string;
  song_ini_absolute_path: string;
  checksum: string;
  message: string;
  absolute_path: string;
};

export type InstrumentValue =
  | "Unknown"
  | "No"
  | "Yes"
  | "Easy"
  | "Medium"
  | "Hard"
  | "Expert"
  | "Error";

export type InstrumentColumnSummary = {
  value: InstrumentValue;
  tooltip: string;
  easy: boolean;
  medium: boolean;
  hard: boolean;
  expert: boolean;
  errors: string[];
};

export type ScannedSongInstruments = {
  guitar: InstrumentColumnSummary;
  bass: InstrumentColumnSummary;
  drums: InstrumentColumnSummary;
  vocals: InstrumentColumnSummary;
  coop_guitar: InstrumentColumnSummary;
  coop_bass: InstrumentColumnSummary;
};

export type ScannedSong = {
  relative_path: string;
  checksum: string;
  folder_absolute_path: string;
  artist: string;
  title: string;
  year: string;
  genre: string;
  game_icon: string;
  has_original_song_ini: boolean;
  is_included: boolean;
  instruments: ScannedSongInstruments;
};

export type SongBackupRow = {
  checksum: string;
  included: boolean;
  artist: string;
  title: string;
  year: string;
  genre: string;
  gameicon: string;
  relative_folder: string;
};

export type SongBackupExportResult = { path: string };

export type ScannedSongMetadata = {
  artist: string;
  title: string;
  year: string;
  genre: string;
  game_icon: string;
};

export type SongIniScanResult = {
  songs_found: number;
  songs_parsed: number;
  songs: ScannedSong[];
  disabled_song_ini_paths: string[];
  faulty_files: FaultySongIniFile[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  song_ini_folder_conflicts: SongIniFolderConflict[];
  content_file_issues: SongContentIssue[];
  disabled_content_file_issues: SongContentIssue[];
  errors: string[];
};

export type SongIniValidationResult = {
  relative_path: string;
  contents: string;
  songs_parsed: number;
  songs: ScannedSong[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  song_ini_folder_conflicts: SongIniFolderConflict[];
  content_file_issues: SongContentIssue[];
};

export type SongContentVerificationResult = {
  songs_parsed: number;
  songs: ScannedSong[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  song_ini_folder_conflicts: SongIniFolderConflict[];
  content_file_issues: SongContentIssue[];
};

export type SongIniDisableResult = {
  relative_path: string;
  disabled_path: string;
  is_included: boolean;
  songs_parsed: number;
};

export type SongIniEnableResult = {
  relative_path: string;
  enabled_path: string;
  is_included: boolean;
  songs_parsed: number;
};

export type SongIniDeleteResult = {
  relative_path: string;
  songs_parsed: number;
};

export type InstrumentAnalyzeMode = "missing" | "errors" | "all";

export type InstrumentAnalyzeProgress = {
  current: number;
  total: number;
  relative_path: string;
  mode: InstrumentAnalyzeMode;
};

export type SongScanProgressPhase =
  | "findingSongs"
  | "readingSongs"
  | "checkingContent"
  | "finishing";

export type SongScanProgress = {
  phase: SongScanProgressPhase;
  current: number;
  total: number;
  relative_path?: string;
};

export type InstrumentAnalyzeResult = {
  songs: ScannedSong[];
  analyzed: number;
  skipped: number;
  errors: number;
};

export type GameIconFile = {
  relative_path: string;
  file_name: string;
  stem: string;
  has_matching_category_ini: boolean;
};

export type GameIconCategoryGroup = {
  folder_relative_path: string;
  folder_absolute_path: string;
  gamelogos: GameIconFile[];
  has_multiple_gamelogos: boolean;
};

export type GameIconCategoryScanResult = {
  groups: GameIconCategoryGroup[];
  needs_fix: boolean;
  official_game_icons: string[];
  custom_game_icons: string[];
  errors: string[];
};

export type GameIconSongFixRow = {
  relative_path: string;
  parent_relative_path: string;
  artist: string;
  title: string;
  invalid_game_icon: string;
  new_game_icon: string;
};

export type GameIconSongFixPreview = {
  rows: GameIconSongFixRow[];
  valid_game_icons: string[];
};

export type GameIconSongFixInput = {
  relative_path: string;
  new_game_icon: string;
};

export type GameIconSongFixApplyResult = {
  applied: number;
  songs_parsed: number;
  songs: ScannedSong[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  song_ini_folder_conflicts: SongIniFolderConflict[];
  content_file_issues: SongContentIssue[];
};

export type OfficialCategoryFile = {
  relative_path: string;
  folder_absolute_path: string;
  checksum: string;
  is_disabled: boolean;
  validation_reasons: string[];
};

export type OfficialCategoryScanResult = {
  categories: OfficialCategoryFile[];
  errors: string[];
};

export type FolderSanitizeRow = {
  relative_path: string;
  absolute_path: string;
  sanitized_name: string;
  sanitized_relative_path: string;
  requires_collision_suffix: boolean;
};

export type FolderSanitizeScanResult = {
  folders: FolderSanitizeRow[];
  errors: string[];
};
