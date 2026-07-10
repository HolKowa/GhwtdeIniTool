export type DeleteFilesPreview = {
  files_to_delete: string[];
  errors: string[];
};

export type DeleteFilesResult = {
  files_deleted: number;
  errors: string[];
};

export type RestoreOriginalSongIniMode =
  | "afterFormatIssueFixes"
  | "beforeFormatIssueFix";

export type RestoreOriginalSongIniResult = {
  files_restored: number;
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

export type DisabledSongConflict = {
  active_path: string;
  disabled_path: string;
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
  folder_absolute_path: string;
  artist: string;
  title: string;
  year: string;
  genre: string;
  game_icon: string;
  has_original_song_ini: boolean;
  instruments: ScannedSongInstruments;
};

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
  faulty_files: FaultySongIniFile[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  disabled_song_conflicts: DisabledSongConflict[];
  content_file_issues: SongContentIssue[];
  errors: string[];
};

export type SongIniValidationResult = {
  relative_path: string;
  contents: string;
  songs_parsed: number;
  songs: ScannedSong[];
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  disabled_song_conflicts: DisabledSongConflict[];
  content_file_issues: SongContentIssue[];
};

export type SongIniDisableResult = {
  relative_path: string;
  disabled_path: string;
  songs_parsed: number;
};

export type SongIniEnableResult = {
  relative_path: string;
  enabled_path: string;
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
