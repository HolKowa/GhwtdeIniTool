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

export type SongIniScanResult = {
  songs_found: number;
  songs_parsed: number;
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
  duplicate_checksum_groups: DuplicateChecksumGroup[];
  disabled_song_conflicts: DisabledSongConflict[];
  content_file_issues: SongContentIssue[];
};

export type SongIniDisableResult = {
  relative_path: string;
  disabled_path: string;
  songs_parsed: number;
};

export type SongIniDeleteResult = {
  relative_path: string;
  songs_parsed: number;
};
