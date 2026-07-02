export const DEFAULT_KEEP_ONLY_FILES_PATTERN =
  "song.ini,*_song.pak.xen,*.fsb.xen,category.ini,*.img.xen";

export function validateKeepOnlyFilesPattern(pattern: string) {
  const invalidCharacterPattern = /[<>:"/\\|?]/;
  const invalidCharacters = '< > : " / \\ | ?';
  const patterns = pattern.split(",");

  for (const rawPattern of patterns) {
    const trimmedPattern = rawPattern.trim();

    if (!trimmedPattern) {
      continue;
    }

    if (invalidCharacterPattern.test(trimmedPattern)) {
      return `Keep patterns match file names only. Remove path or reserved characters: ${invalidCharacters}.`;
    }
  }

  return "";
}
