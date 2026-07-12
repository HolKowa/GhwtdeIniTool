use serde::Serialize;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

const DIFFICULTIES: [&str; 4] = ["Easy", "Medium", "Hard", "Expert"];
const PAK_HEADER_SIZE: usize = 32;
const PAK_FTYPE_LAST: u32 = 0x2cb3ef3b;
const PAK_FTYPE_LAST_TH: u32 = 0xb524565f;
const PAK_FLAG_HAS_FILENAME: u32 = 0x20;
const PAK_INLINE_FILENAME_BYTES: usize = 160;
const PAK_KNOWN_FLAGS_MASK: u32 = 0x20;
const SECTION_ARRAY: u32 = 0x00200c00;
const ARRAY_INTEGER: u32 = 0x00010100;
const ARRAY_FLOAT: u32 = 0x00010200;
const ARRAY_QBKEY: u32 = 0x00010d00;
const FLOATS: u32 = 0x00010000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongPakAnalysis {
    checksum: String,
    pak_path: String,
    pak_readable: bool,
    pak_reader_format: String,
    pak_reader_warnings: Vec<String>,
    main_qb_found: bool,
    script_qb_found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    main_qb_lookup: Option<MainQbLookup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    main_qb_matched_entry_hash: Option<String>,
    main_qb_matching_section_count: usize,
    hashes: SongPakHashes,
    instruments: SongPakInstruments,
    details: SongPakDetails,
    errors: Vec<String>,
    warnings: Vec<String>,
    pak_entries: Vec<SongPakEntrySummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SongPakHashes {
    main_qb: NamedHash,
    script_qb: NamedHash,
}

#[derive(Clone, Debug, Serialize)]
struct NamedHash {
    name: String,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    matched_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SongPakInstruments {
    guitar: DifficultySummary,
    bass: DifficultySummary,
    drums: DifficultySummary,
    coop: CoopSummary,
    vocals: VocalSummary,
    any: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
struct DifficultySummary {
    easy: bool,
    medium: bool,
    hard: bool,
    expert: bool,
    any: bool,
}

#[derive(Debug, Serialize)]
struct CoopSummary {
    guitar: DifficultySummary,
    rhythm: DifficultySummary,
    any: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct VocalSummary {
    supported: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    section_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    section_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    found: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_count: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SongPakDetails {
    guitar: DifficultyDetails,
    bass: DifficultyDetails,
    drums: DifficultyDetails,
    coop: CoopDetails,
    vocals: VocalSummary,
    section_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DifficultyDetails {
    easy: DifficultyDetail,
    medium: DifficultyDetail,
    hard: DifficultyDetail,
    expert: DifficultyDetail,
    any: bool,
    label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DifficultyDetail {
    supported: bool,
    section_name: String,
    section_hash: String,
    found: bool,
    value_count: usize,
}

#[derive(Debug, Serialize)]
struct CoopDetails {
    guitar: DifficultyDetails,
    rhythm: DifficultyDetails,
    any: bool,
}

#[derive(Debug, Serialize)]
struct SongPakEntrySummary {
    hashes: Vec<String>,
    extension: String,
    size: usize,
}

#[derive(Debug)]
struct PakDiagnostics {
    format: String,
    warnings: Vec<String>,
    fallbacks: Vec<String>,
}

#[derive(Debug)]
struct PakEntries {
    entries: Vec<PakEntry>,
    diagnostics: PakDiagnostics,
}

#[derive(Debug)]
struct PakEntry {
    start: u32,
    full_name: u32,
    name_sum: u32,
    data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum MainQbLookup {
    SongsPath,
    RootPath,
    SectionDiscovery,
}

struct MainQbMatch<'a> {
    entry: &'a PakEntry,
    lookup: MainQbLookup,
    matched_name: Option<String>,
    sections: HashMap<String, QbSection>,
    matching_section_count: usize,
}

#[derive(Clone, Debug)]
struct QbSection {
    values: Option<Vec<QbValue>>,
    dummy_floats: bool,
}

#[derive(Clone, Debug)]
enum QbValue {
    U32(u32),
    F32,
}

pub fn analyze_song_pak(pak_path: &str, checksum: &str) -> Result<SongPakAnalysis, String> {
    if pak_path.trim().is_empty() {
        return Err("Missing pak path.".to_string());
    }

    if checksum.trim().is_empty() {
        return Err("Missing checksum.".to_string());
    }

    let resolved_pak_path =
        fs::canonicalize(pak_path).map_err(|err| format_pak_path_error(pak_path, err))?;

    if !resolved_pak_path.is_file() {
        return Err(format!(
            "Pak file does not exist: {}",
            resolved_pak_path.display()
        ));
    }

    Ok(analyze_existing_song_pak(
        &resolved_pak_path,
        checksum.trim(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct SongPakInstrumentAvailability {
    pub(crate) guitar: DifficultyAvailability,
    pub(crate) bass: DifficultyAvailability,
    pub(crate) drums: DifficultyAvailability,
    pub(crate) coop_guitar: DifficultyAvailability,
    pub(crate) coop_bass: DifficultyAvailability,
    pub(crate) vocals: VocalAvailability,
    pub(crate) errors: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DifficultyAvailability {
    pub(crate) easy: bool,
    pub(crate) medium: bool,
    pub(crate) hard: bool,
    pub(crate) expert: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VocalAvailability {
    pub(crate) supported: bool,
}

pub(crate) fn analyze_song_pak_instruments(
    pak_path: &str,
    checksum: &str,
) -> Result<SongPakInstrumentAvailability, String> {
    let analysis = analyze_song_pak(pak_path, checksum)?;

    Ok(SongPakInstrumentAvailability {
        guitar: difficulty_availability(&analysis.instruments.guitar),
        bass: difficulty_availability(&analysis.instruments.bass),
        drums: difficulty_availability(&analysis.instruments.drums),
        coop_guitar: difficulty_availability(&analysis.instruments.coop.guitar),
        coop_bass: difficulty_availability(&analysis.instruments.coop.rhythm),
        vocals: VocalAvailability {
            supported: analysis.instruments.vocals.supported,
        },
        errors: analysis.errors,
    })
}

fn difficulty_availability(summary: &DifficultySummary) -> DifficultyAvailability {
    DifficultyAvailability {
        easy: summary.easy,
        medium: summary.medium,
        hard: summary.hard,
        expert: summary.expert,
    }
}

fn analyze_existing_song_pak(pak_path: &Path, checksum: &str) -> SongPakAnalysis {
    let main_qb_candidates = qb_lookup_candidates(checksum, ".mid.qb");
    let script_qb_candidates = qb_lookup_candidates(checksum, "_song_scripts.qb");
    let mut result = empty_analysis(
        checksum,
        pak_path,
        &main_qb_candidates[0],
        &script_qb_candidates[0],
    );

    let entries = match read_pak_entries(pak_path) {
        Ok(entries) => entries,
        Err(err) => {
            result.errors.push(format!("Failed to read pak: {err}"));
            return result;
        }
    };

    result.pak_readable = true;
    result.pak_reader_format = entries.diagnostics.format;
    result.pak_reader_warnings = entries.diagnostics.warnings;
    result.warnings.extend(
        result
            .pak_reader_warnings
            .iter()
            .map(|warning| format!("Pak reader: {warning}")),
    );
    result.warnings.extend(
        entries
            .diagnostics
            .fallbacks
            .iter()
            .map(|fallback| format!("Pak reader fallback: {fallback}")),
    );

    let main_match = find_qb_entry(&entries.entries, &main_qb_candidates);
    let script_match = find_qb_entry(&entries.entries, &script_qb_candidates);

    result.script_qb_found = script_match.is_some();
    result.hashes.script_qb.matched_name =
        script_match.map(|(_, candidate, _)| candidate.name.clone());
    result.pak_entries = entries
        .entries
        .iter()
        .map(|entry| SongPakEntrySummary {
            hashes: get_entry_hashes(entry),
            extension: String::new(),
            size: entry.data.len(),
        })
        .collect();

    let resolved_main_qb = if let Some((candidate_index, candidate, entry)) = main_match {
        let sections = match parse_qb_sections(&entry.data) {
            Ok(sections) => sections,
            Err(err) => {
                result
                    .errors
                    .push(format!("Failed to parse main chart QB: {err}"));
                return result;
            }
        };

        MainQbMatch {
            entry,
            lookup: if candidate_index == 0 {
                MainQbLookup::SongsPath
            } else {
                MainQbLookup::RootPath
            },
            matched_name: Some(candidate.name.clone()),
            matching_section_count: matching_chart_section_count(checksum, &sections),
            sections,
        }
    } else if let Some(discovered) = discover_main_chart_qb(&entries.entries, checksum) {
        discovered
    } else {
        result.errors.push(format!(
            "No main chart QB found for {} ({}), or {} ({}).",
            main_qb_candidates[0].name,
            main_qb_candidates[0].hash,
            main_qb_candidates[1].name,
            main_qb_candidates[1].hash,
        ));
        return result;
    };

    result.main_qb_found = true;
    result.main_qb_lookup = Some(resolved_main_qb.lookup);
    result.main_qb_matched_entry_hash = Some(hex(resolved_main_qb.entry.full_name));
    result.main_qb_matching_section_count = resolved_main_qb.matching_section_count;
    result.hashes.main_qb.matched_name = resolved_main_qb.matched_name;

    let sections = resolved_main_qb.sections;

    let guitar = create_difficulty_result(checksum, "guitar", "song", &sections);
    let bass = create_difficulty_result(checksum, "bass", "song_rhythm", &sections);
    let drums = create_difficulty_result(checksum, "drums", "song_drum", &sections);
    let guitar_coop =
        create_difficulty_result(checksum, "guitar coop", "song_guitarcoop", &sections);
    let rhythm_coop =
        create_difficulty_result(checksum, "rhythm coop", "song_rhythmcoop", &sections);

    let vocal_name = format!("{checksum}_song_vocals");
    let vocal_hash = qb_key_hex(&vocal_name);
    let vocal_section = sections.get(&vocal_hash);
    let vocal_summary = VocalSummary {
        supported: is_real_vocal_array(vocal_section),
        section_name: vocal_name,
        section_hash: vocal_hash,
        found: Some(vocal_section.is_some()),
        value_count: Some(value_count(vocal_section)),
    };

    result.instruments.guitar = summarize_difficulty_result(&guitar);
    result.instruments.bass = summarize_difficulty_result(&bass);
    result.instruments.drums = summarize_difficulty_result(&drums);
    result.instruments.coop = CoopSummary {
        guitar: summarize_difficulty_result(&guitar_coop),
        rhythm: summarize_difficulty_result(&rhythm_coop),
        any: guitar_coop.any || rhythm_coop.any,
    };
    result.instruments.vocals = vocal_summary.clone();
    result.instruments.any = result.instruments.guitar.any
        || result.instruments.bass.any
        || result.instruments.drums.any
        || result.instruments.coop.any
        || result.instruments.vocals.supported;

    result.details = SongPakDetails {
        guitar,
        bass,
        drums,
        coop: CoopDetails {
            guitar: guitar_coop.clone(),
            rhythm: rhythm_coop.clone(),
            any: guitar_coop.any || rhythm_coop.any,
        },
        vocals: vocal_summary,
        section_count: sections.len(),
    };

    result
}

fn empty_analysis(
    checksum: &str,
    pak_path: &Path,
    main_qb: &NamedHash,
    script_qb: &NamedHash,
) -> SongPakAnalysis {
    SongPakAnalysis {
        checksum: checksum.to_string(),
        pak_path: pak_path.to_string_lossy().into_owned(),
        pak_readable: false,
        pak_reader_format: String::new(),
        pak_reader_warnings: Vec::new(),
        main_qb_found: false,
        script_qb_found: false,
        main_qb_lookup: None,
        main_qb_matched_entry_hash: None,
        main_qb_matching_section_count: 0,
        hashes: SongPakHashes {
            main_qb: main_qb.clone(),
            script_qb: script_qb.clone(),
        },
        instruments: SongPakInstruments {
            guitar: DifficultySummary::default(),
            bass: DifficultySummary::default(),
            drums: DifficultySummary::default(),
            coop: CoopSummary {
                guitar: DifficultySummary::default(),
                rhythm: DifficultySummary::default(),
                any: false,
            },
            vocals: VocalSummary::default(),
            any: false,
        },
        details: SongPakDetails {
            guitar: empty_difficulty_details("guitar"),
            bass: empty_difficulty_details("bass"),
            drums: empty_difficulty_details("drums"),
            coop: CoopDetails {
                guitar: empty_difficulty_details("guitar coop"),
                rhythm: empty_difficulty_details("rhythm coop"),
                any: false,
            },
            vocals: VocalSummary::default(),
            section_count: 0,
        },
        errors: Vec::new(),
        warnings: Vec::new(),
        pak_entries: Vec::new(),
    }
}

fn qb_lookup_candidates(checksum: &str, suffix: &str) -> [NamedHash; 2] {
    let songs_name = format!("songs/{checksum}{suffix}");
    let root_name = format!("{checksum}{suffix}");

    [songs_name, root_name].map(|name| NamedHash {
        hash: qb_key_hex(&name),
        name,
        matched_name: None,
    })
}

fn empty_difficulty_details(label: &str) -> DifficultyDetails {
    let empty = DifficultyDetail {
        supported: false,
        section_name: String::new(),
        section_hash: String::new(),
        found: false,
        value_count: 0,
    };

    DifficultyDetails {
        easy: empty.clone(),
        medium: empty.clone(),
        hard: empty.clone(),
        expert: empty,
        any: false,
        label: label.to_string(),
    }
}

fn create_difficulty_result(
    checksum: &str,
    label: &str,
    section_name: &str,
    sections: &HashMap<String, QbSection>,
) -> DifficultyDetails {
    let details = DIFFICULTIES.map(|difficulty| {
        let name = format!("{checksum}_{section_name}_{difficulty}");
        let hash = qb_key_hex(&name);
        let section = sections.get(&hash);

        DifficultyDetail {
            supported: is_real_playable_array(section),
            section_name: name,
            section_hash: hash,
            found: section.is_some(),
            value_count: value_count(section),
        }
    });

    let easy = details[0].clone();
    let medium = details[1].clone();
    let hard = details[2].clone();
    let expert = details[3].clone();
    let any = details.iter().any(|detail| detail.supported);

    DifficultyDetails {
        easy,
        medium,
        hard,
        expert,
        any,
        label: label.to_string(),
    }
}

fn summarize_difficulty_result(result: &DifficultyDetails) -> DifficultySummary {
    DifficultySummary {
        easy: result.easy.supported,
        medium: result.medium.supported,
        hard: result.hard.supported,
        expert: result.expert.supported,
        any: result.any,
    }
}

fn is_real_playable_array(section: Option<&QbSection>) -> bool {
    let Some(section) = section else {
        return false;
    };
    let Some(values) = &section.values else {
        return false;
    };

    !section.dummy_floats && values.len() >= 2 && values.len() % 2 == 0
}

fn is_real_vocal_array(section: Option<&QbSection>) -> bool {
    let Some(values) = section.and_then(|section| section.values.as_ref()) else {
        return false;
    };

    if values.len() < 3 {
        return false;
    }

    if values.len() == 3
        && matches!(values[0], QbValue::U32(10000))
        && matches!(values[1], QbValue::U32(5000))
        && matches!(values[2], QbValue::U32(58))
    {
        return false;
    }

    values.len() % 3 == 0
}

fn value_count(section: Option<&QbSection>) -> usize {
    section
        .and_then(|section| section.values.as_ref())
        .map_or(0, Vec::len)
}

fn read_pak_entries(pak_path: &Path) -> Result<PakEntries, String> {
    let data = fs::read(pak_path).map_err(|err| err.to_string())?;
    read_pak_entries_from_bytes(&data)
}

fn read_pak_entries_from_bytes(data: &[u8]) -> Result<PakEntries, String> {
    let mut reader = BinaryReader::new(data);
    let mut entries = Vec::new();
    let mut diagnostics = PakDiagnostics {
        format: "gh-header-relative".to_string(),
        warnings: Vec::new(),
        fallbacks: Vec::new(),
    };

    while reader.can_read(PAK_HEADER_SIZE) {
        let parsed = parse_pak_header_candidate(&mut reader, data.len(), &mut diagnostics)?;

        let Some(file) = parsed else {
            break;
        };

        if entries.is_empty() && file.start == 0 {
            diagnostics.warnings.push(
                "First entry starts at 0; this may be a PAB/CAS-style pak. Sidecar PAB data is not supported by this standalone reader.".to_string(),
            );
        }

        entries.push(file);
    }

    Ok(PakEntries {
        entries,
        diagnostics,
    })
}

fn parse_pak_header_candidate(
    reader: &mut BinaryReader<'_>,
    data_len: usize,
    diagnostics: &mut PakDiagnostics,
) -> Result<Option<PakEntry>, String> {
    if !reader.can_read(PAK_HEADER_SIZE) {
        return Ok(None);
    }

    let header_start = reader.tell();
    let entry_type = reader.u32()?;

    if is_last_marker(entry_type) {
        return Ok(None);
    }

    let start = reader.u32()?;
    let size = reader.u32()?;
    let _pak_full_filename_key = reader.u32()?;
    let full_name = reader.u32()?;
    let name_sum = reader.u32()?;
    let _parent = reader.u32()?;
    let flags = reader.u32()?;

    if flags & !PAK_KNOWN_FLAGS_MASK != 0 {
        diagnostics.warnings.push(format!(
            "Entry {} has unknown flag bits {}.",
            hex(full_name),
            hex(flags & !PAK_KNOWN_FLAGS_MASK)
        ));
    }

    if full_name == 0 && name_sum == 0 {
        diagnostics.warnings.push(format!(
            "Entry at header {header_start} has zero fullName and nameSum."
        ));
    }

    if flags & PAK_FLAG_HAS_FILENAME != 0 {
        if !reader.can_read(PAK_INLINE_FILENAME_BYTES) {
            return Err(format!(
                "Pak entry {} has inline filename flag, but the filename block exceeds file bounds.",
                hex(full_name)
            ));
        }

        reader.chunk(PAK_INLINE_FILENAME_BYTES)?;
        diagnostics.warnings.push(format!(
            "Entry {} skipped {PAK_INLINE_FILENAME_BYTES}-byte inline filename block.",
            hex(full_name)
        ));
    }

    let data_start =
        choose_data_start(header_start, start, size, full_name, data_len, diagnostics)?;
    let data_end = data_start + size as usize;

    Ok(Some(PakEntry {
        start,
        full_name,
        name_sum,
        data: reader.buffer[data_start..data_end].to_vec(),
    }))
}

fn choose_data_start(
    header_start: usize,
    start: u32,
    size: u32,
    full_name: u32,
    data_len: usize,
    diagnostics: &mut PakDiagnostics,
) -> Result<usize, String> {
    let candidates = [
        (
            "header-relative",
            header_start.saturating_add(start as usize),
        ),
        ("absolute", start as usize),
    ];

    for (name, data_start) in candidates {
        if data_start + size as usize <= data_len {
            if name != "header-relative" {
                diagnostics.fallbacks.push(format!(
                    "Entry {} used {name} offset strategy.",
                    hex(full_name)
                ));
            }

            return Ok(data_start);
        }
    }

    Err(format!(
        "Pak entry {} exceeds file bounds: start={start}, headerStart={header_start}, size={size}, fileSize={data_len}.",
        hex(full_name)
    ))
}

fn find_pak_entry<'a>(entries: &'a [PakEntry], expected_hash: &str) -> Option<&'a PakEntry> {
    let wanted = normalize_hash(expected_hash);

    entries
        .iter()
        .find(|entry| get_entry_hashes(entry).iter().any(|hash| hash == &wanted))
}

fn find_qb_entry<'a>(
    entries: &'a [PakEntry],
    candidates: &'a [NamedHash],
) -> Option<(usize, &'a NamedHash, &'a PakEntry)> {
    candidates
        .iter()
        .enumerate()
        .find_map(|(index, candidate)| {
            find_pak_entry(entries, &candidate.hash).map(|entry| (index, candidate, entry))
        })
}

fn discover_main_chart_qb<'a>(entries: &'a [PakEntry], checksum: &str) -> Option<MainQbMatch<'a>> {
    let mut best_match = None;

    for entry in entries.iter().filter(|entry| is_plausible_qb(&entry.data)) {
        let Ok(sections) = parse_qb_sections(&entry.data) else {
            continue;
        };

        let matching_section_count = matching_chart_section_count(checksum, &sections);
        let playable_section_count = playable_chart_section_count(checksum, &sections);
        if playable_section_count == 0 && matching_section_count < 2 {
            continue;
        }

        let candidate = MainQbMatch {
            entry,
            lookup: MainQbLookup::SectionDiscovery,
            matched_name: None,
            sections,
            matching_section_count,
        };

        let candidate_score = (playable_section_count, matching_section_count);
        let best_score = best_match.as_ref().map(|best: &MainQbMatch<'_>| {
            (
                playable_chart_section_count(checksum, &best.sections),
                best.matching_section_count,
            )
        });
        if best_score.is_none_or(|score| candidate_score > score) {
            best_match = Some(candidate);
        }
    }

    best_match
}

fn is_plausible_qb(data: &[u8]) -> bool {
    data.len() >= 8
        && data.chunks_exact(4).any(|chunk| {
            u32::from_be_bytes(chunk.try_into().expect("chunk has four bytes")) == SECTION_ARRAY
        })
}

fn matching_chart_section_count(checksum: &str, sections: &HashMap<String, QbSection>) -> usize {
    expected_chart_section_hashes(checksum)
        .iter()
        .filter(|hash| sections.contains_key(*hash))
        .count()
}

fn playable_chart_section_count(checksum: &str, sections: &HashMap<String, QbSection>) -> usize {
    ["song", "song_rhythm", "song_drum"]
        .into_iter()
        .flat_map(|section_name| {
            DIFFICULTIES.map(|difficulty| format!("{checksum}_{section_name}_{difficulty}"))
        })
        .filter(|name| is_real_playable_array(sections.get(&qb_key_hex(name))))
        .count()
}

fn expected_chart_section_hashes(checksum: &str) -> Vec<String> {
    let mut names = ["song", "song_rhythm", "song_drum"]
        .into_iter()
        .flat_map(|section_name| {
            DIFFICULTIES.map(|difficulty| format!("{checksum}_{section_name}_{difficulty}"))
        })
        .map(|name| qb_key_hex(&name))
        .collect::<Vec<_>>();
    names.push(qb_key_hex(&format!("{checksum}_song_vocals")));
    names
}

fn get_entry_hashes(entry: &PakEntry) -> Vec<String> {
    [entry.full_name, entry.name_sum]
        .into_iter()
        .map(hex)
        .filter(|hash| !hash.is_empty())
        .collect()
}

fn is_last_marker(entry_type: u32) -> bool {
    entry_type == PAK_FTYPE_LAST || entry_type == PAK_FTYPE_LAST_TH
}

fn parse_qb_sections(buffer: &[u8]) -> Result<HashMap<String, QbSection>, String> {
    let mut reader = BinaryReader::new(buffer);
    let mut sections = HashMap::new();

    reader.u32()?;
    reader.u32()?;

    while reader.can_read(4) {
        let start = reader.tell();
        let item_type = reader.u32()?;

        if item_type == SECTION_ARRAY {
            if let Ok(section) = read_section_array(&mut reader) {
                sections.insert(section.0, section.1);
            }
        }

        reader.seek(start + 4);
    }

    Ok(sections)
}

fn read_section_array(reader: &mut BinaryReader<'_>) -> Result<(String, QbSection), String> {
    let id = reader.u32()?;
    reader.u32()?;
    let pointer = reader.u32()?;
    reader.u32()?;

    let child_start = if pointer == 0 {
        reader.tell()
    } else {
        pointer as usize
    };

    if child_start >= reader.buffer.len() {
        return Ok((
            hex(id),
            QbSection {
                values: None,
                dummy_floats: false,
            },
        ));
    }

    reader.seek(child_start);
    let child_type = reader.u32()?;

    if [ARRAY_INTEGER, ARRAY_FLOAT, ARRAY_QBKEY].contains(&child_type) {
        return Ok((
            hex(id),
            QbSection {
                values: Some(read_primitive_array(reader, child_type)?),
                dummy_floats: false,
            },
        ));
    }

    if child_type == FLOATS {
        reader.f32()?;
        reader.f32()?;

        return Ok((
            hex(id),
            QbSection {
                values: Some(vec![QbValue::F32, QbValue::F32]),
                dummy_floats: true,
            },
        ));
    }

    Ok((
        hex(id),
        QbSection {
            values: None,
            dummy_floats: false,
        },
    ))
}

fn read_primitive_array(reader: &mut BinaryReader<'_>, kind: u32) -> Result<Vec<QbValue>, String> {
    let mut count = reader.u32()?;

    if count > 50000 {
        reader.seek(reader.tell() - 4);
        count = 1;
    }

    if !(count == 1 && kind == ARRAY_QBKEY) {
        reader.u32()?;
    }

    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if kind == ARRAY_FLOAT {
            reader.f32()?;
            values.push(QbValue::F32);
        } else {
            values.push(QbValue::U32(reader.u32()?));
        }
    }

    Ok(values)
}

fn qb_key_hex(text: &str) -> String {
    hex(qb_key(text))
}

fn qb_key(text: &str) -> u32 {
    if let Some(hex_text) = text.strip_prefix("0x") {
        return u32::from_str_radix(hex_text, 16).unwrap_or(0);
    }

    let normalized = text.replace('/', "\\").to_lowercase();
    let mut crc = 0xffff_ffff;

    for byte in normalized.as_bytes() {
        let index = ((crc ^ u32::from(*byte)) & 0xff) as usize;
        crc = CRC32_TABLE[index] ^ (crc >> 8);
    }

    crc
}

fn normalize_hash(value: &str) -> String {
    let text = value.trim().to_lowercase();

    if text.is_empty() {
        return String::new();
    }

    if let Some(hash) = text.strip_prefix("0x") {
        return format!("0x{:0>8}", hash);
    }

    if text.len() == 8 && text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return format!("0x{text}");
    }

    text
}

fn hex(value: u32) -> String {
    format!("0x{value:08x}")
}

fn format_pak_path_error(pak_path: &str, err: io::Error) -> String {
    let path = PathBuf::from(pak_path);
    if err.kind() == io::ErrorKind::NotFound || !path.exists() {
        format!("Pak file does not exist: {}", path.display())
    } else {
        format!("Failed to resolve pak path {}: {err}", path.display())
    }
}

struct BinaryReader<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    fn tell(&self) -> usize {
        self.offset
    }

    fn seek(&mut self, offset: usize) {
        self.offset = offset;
    }

    fn can_read(&self, size: usize) -> bool {
        self.offset + size <= self.buffer.len()
    }

    fn u32(&mut self) -> Result<u32, String> {
        if !self.can_read(4) {
            return Err("Unexpected end of file while reading u32.".to_string());
        }

        let value = u32::from_be_bytes(
            self.buffer[self.offset..self.offset + 4]
                .try_into()
                .expect("slice length was checked"),
        );
        self.offset += 4;
        Ok(value)
    }

    fn f32(&mut self) -> Result<f32, String> {
        if !self.can_read(4) {
            return Err("Unexpected end of file while reading f32.".to_string());
        }

        let value = f32::from_be_bytes(
            self.buffer[self.offset..self.offset + 4]
                .try_into()
                .expect("slice length was checked"),
        );
        self.offset += 4;
        Ok(value)
    }

    fn chunk(&mut self, size: usize) -> Result<&'a [u8], String> {
        if !self.can_read(size) {
            return Err(format!(
                "Unexpected end of file while reading {size} bytes."
            ));
        }

        let chunk = &self.buffer[self.offset..self.offset + size];
        self.offset += size;
        Ok(chunk)
    }
}

const CRC32_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn qb_key_matches_sample_hashes() {
        assert_eq!(qb_key_hex("songs/sk8erboi_rr.mid.qb"), "0x59131bdb");
        assert_eq!(
            qb_key_hex("songs/sk8erboi_rr_song_scripts.qb"),
            "0x2dfe239e"
        );
        assert_eq!(qb_key_hex("sk8erboi_rr_song_vocals"), "0x42d71857");
        assert_eq!(qb_key_hex("songs\\sk8erboi_rr.mid.qb"), "0x59131bdb");
    }

    #[test]
    fn normalize_hash_accepts_supported_forms() {
        assert_eq!(normalize_hash("59131bdb"), "0x59131bdb");
        assert_eq!(normalize_hash("0x59131bdb"), "0x59131bdb");
        assert_eq!(normalize_hash("0x123"), "0x00000123");
        assert_eq!(normalize_hash("song_name"), "song_name");
    }

    #[test]
    fn reads_synthetic_pak_entries() {
        let main_qb_hash = qb_key("songs/sample.mid.qb");
        let script_qb_hash = qb_key("songs/sample_song_scripts.qb");
        let pak = synthetic_pak(&[(main_qb_hash, &[1, 2, 3]), (script_qb_hash, &[4, 5])]);
        let entries = read_pak_entries_from_bytes(&pak).expect("pak should parse");

        assert_eq!(entries.diagnostics.format, "gh-header-relative");
        assert!(entries.diagnostics.warnings.is_empty());
        assert_eq!(entries.entries.len(), 2);
        assert_eq!(entries.entries[0].data, vec![1, 2, 3]);
        assert_eq!(entries.entries[1].data, vec![4, 5]);
        assert!(find_pak_entry(&entries.entries, &hex(main_qb_hash)).is_some());
        assert!(find_pak_entry(&entries.entries, &hex(script_qb_hash)).is_some());
    }

    #[test]
    fn song_path_qb_entries_take_precedence_over_root_entries() {
        let songs_main =
            synthetic_qb(&[(qb_key("sample_song_Easy"), ARRAY_INTEGER, &[100, 1], false)]);
        let root_main = synthetic_qb(&[]);
        let pak = synthetic_pak(&[
            (qb_key("sample.mid.qb"), root_main.as_slice()),
            (qb_key("sample_song_scripts.qb"), &[1]),
            (qb_key("songs/sample.mid.qb"), songs_main.as_slice()),
            (qb_key("songs/sample_song_scripts.qb"), &[2]),
        ]);

        let analysis = analyze_synthetic_pak(&pak, "sample");

        assert!(analysis.errors.is_empty());
        assert_eq!(
            analysis.hashes.main_qb.matched_name.as_deref(),
            Some("songs/sample.mid.qb")
        );
        assert_eq!(
            analysis.hashes.script_qb.matched_name.as_deref(),
            Some("songs/sample_song_scripts.qb")
        );
        assert_eq!(analysis.main_qb_lookup, Some(MainQbLookup::SongsPath));
        let songs_main_hash = hex(qb_key("songs/sample.mid.qb"));
        assert_eq!(
            analysis.main_qb_matched_entry_hash.as_deref(),
            Some(songs_main_hash.as_str())
        );
        assert_eq!(analysis.main_qb_matching_section_count, 1);
        assert!(analysis.instruments.guitar.easy);
    }

    #[test]
    fn root_level_qb_entries_are_used_for_ghwt_style_song_paks() {
        let checksum = "DLC1057";
        let main_qb = synthetic_qb(&[
            (qb_key("DLC1057_song_Easy"), ARRAY_INTEGER, &[100, 1], false),
            (
                qb_key("DLC1057_song_rhythm_Easy"),
                ARRAY_INTEGER,
                &[100, 1],
                false,
            ),
            (
                qb_key("DLC1057_song_drum_Easy"),
                ARRAY_INTEGER,
                &[100, 1],
                false,
            ),
            (
                qb_key("DLC1057_song_vocals"),
                ARRAY_INTEGER,
                &[100, 50, 60],
                false,
            ),
        ]);
        let pak = synthetic_pak(&[
            (qb_key("DLC1057.mid.qb"), main_qb.as_slice()),
            (qb_key("DLC1057_song_scripts.qb"), &[1]),
        ]);

        let analysis = analyze_synthetic_pak(&pak, checksum);

        assert_eq!(qb_key_hex("DLC1057.mid.qb"), "0xb6cc9a4e");
        assert_eq!(qb_key_hex("DLC1057_song_scripts.qb"), "0x88f47b50");
        assert!(analysis.errors.is_empty());
        assert!(analysis.main_qb_found);
        assert!(analysis.script_qb_found);
        assert_eq!(
            analysis.hashes.main_qb.matched_name.as_deref(),
            Some("DLC1057.mid.qb")
        );
        assert_eq!(
            analysis.hashes.script_qb.matched_name.as_deref(),
            Some("DLC1057_song_scripts.qb")
        );
        assert_eq!(analysis.main_qb_lookup, Some(MainQbLookup::RootPath));
        let root_main_hash = hex(qb_key("DLC1057.mid.qb"));
        assert_eq!(
            analysis.main_qb_matched_entry_hash.as_deref(),
            Some(root_main_hash.as_str())
        );
        assert_eq!(analysis.main_qb_matching_section_count, 4);
        assert!(analysis.instruments.guitar.easy);
        assert!(analysis.instruments.bass.easy);
        assert!(analysis.instruments.drums.easy);
        assert!(analysis.instruments.vocals.supported);
    }

    #[test]
    fn section_discovery_selects_the_strongest_checksum_specific_chart() {
        let weak_chart = synthetic_qb(&[
            (qb_key("sample_song_Easy"), ARRAY_INTEGER, &[100], false),
            (
                qb_key("sample_song_rhythm_Easy"),
                ARRAY_INTEGER,
                &[100],
                false,
            ),
        ]);
        let strong_chart = synthetic_qb(&[
            (qb_key("sample_song_Easy"), ARRAY_INTEGER, &[100, 1], false),
            (
                qb_key("sample_song_rhythm_Easy"),
                ARRAY_INTEGER,
                &[100, 1],
                false,
            ),
            (
                qb_key("sample_song_drum_Easy"),
                ARRAY_INTEGER,
                &[100, 1],
                false,
            ),
            (
                qb_key("sample_song_vocals"),
                ARRAY_INTEGER,
                &[100, 50, 60],
                false,
            ),
        ]);
        let decoy_chart =
            synthetic_qb(&[(qb_key("other_song_Easy"), ARRAY_INTEGER, &[100, 1], false)]);
        let pak = synthetic_pak(&[
            (0x1111_1111, weak_chart.as_slice()),
            (0x3333_3333, decoy_chart.as_slice()),
            (0x2222_2222, strong_chart.as_slice()),
        ]);

        let analysis = analyze_synthetic_pak(&pak, "sample");

        assert!(analysis.errors.is_empty());
        assert!(analysis.main_qb_found);
        assert_eq!(
            analysis.main_qb_lookup,
            Some(MainQbLookup::SectionDiscovery)
        );
        assert_eq!(
            analysis.main_qb_matched_entry_hash.as_deref(),
            Some("0x22222222")
        );
        assert_eq!(analysis.main_qb_matching_section_count, 4);
        assert!(analysis.hashes.main_qb.matched_name.is_none());
        assert!(analysis.instruments.guitar.easy);
        assert!(analysis.instruments.bass.easy);
        assert!(analysis.instruments.drums.easy);
        assert!(analysis.instruments.vocals.supported);
    }

    #[test]
    fn parses_qb_sections_and_detects_instruments() {
        let qb = synthetic_qb(&[
            (
                qb_key("sample_song_Easy"),
                ARRAY_INTEGER,
                &[100, 1, 200, 2],
                false,
            ),
            (qb_key("sample_song_Medium"), ARRAY_INTEGER, &[100], false),
            (qb_key("sample_song_Hard"), FLOATS, &[], true),
            (
                qb_key("sample_song_vocals"),
                ARRAY_INTEGER,
                &[10000, 5000, 58],
                false,
            ),
        ]);

        let sections = parse_qb_sections(&qb).expect("qb should parse");
        let guitar = create_difficulty_result("sample", "guitar", "song", &sections);

        assert_eq!(sections.len(), 4);
        assert!(guitar.easy.supported);
        assert!(!guitar.medium.supported);
        assert!(!guitar.hard.supported);
        assert!(!guitar.expert.found);
        assert!(!is_real_vocal_array(
            sections.get(&qb_key_hex("sample_song_vocals"))
        ));
    }

    #[test]
    fn detects_real_vocal_arrays() {
        let qb = synthetic_qb(&[(
            qb_key("sample_song_vocals"),
            ARRAY_INTEGER,
            &[100, 50, 60, 200, 75, 62],
            false,
        )]);
        let sections = parse_qb_sections(&qb).expect("qb should parse");

        assert!(is_real_vocal_array(
            sections.get(&qb_key_hex("sample_song_vocals"))
        ));
    }

    #[test]
    fn real_sample_matches_reference_when_available() {
        let pak_path = Path::new(
            "../../MODS_medium/MODS/Rock Revolution/Avril Lavigne - Sk8er Boi/Content/ask8erboi_rr_song.pak.xen",
        );

        if !pak_path.exists() {
            return;
        }

        let analysis = analyze_song_pak(&pak_path.to_string_lossy(), "sk8erboi_rr")
            .expect("real sample should analyze");

        assert!(analysis.errors.is_empty());
        assert!(analysis.warnings.is_empty());
        assert_eq!(analysis.hashes.main_qb.hash, "0x59131bdb");
        assert_eq!(analysis.hashes.script_qb.hash, "0x2dfe239e");
        assert!(analysis.main_qb_found);
        assert!(analysis.script_qb_found);
        assert!(analysis.instruments.guitar.easy);
        assert!(analysis.instruments.guitar.medium);
        assert!(analysis.instruments.guitar.hard);
        assert!(analysis.instruments.guitar.expert);
        assert!(analysis.instruments.bass.any);
        assert!(analysis.instruments.drums.any);
        assert!(!analysis.instruments.coop.any);
        assert!(!analysis.instruments.vocals.supported);
        assert_eq!(analysis.instruments.vocals.value_count, Some(3));
        assert_eq!(analysis.details.guitar.easy.value_count, 422);
        assert_eq!(analysis.details.guitar.expert.value_count, 1380);
        assert_eq!(analysis.details.bass.expert.value_count, 1434);
        assert_eq!(analysis.details.drums.expert.value_count, 1914);
        assert_eq!(analysis.details.section_count, 173);
        assert_eq!(analysis.pak_entries.len(), 2);
        assert_eq!(analysis.pak_entries[0].hashes[0], "0x59131bdb");
        assert_eq!(analysis.pak_entries[0].size, 70564);
        assert_eq!(analysis.pak_entries[1].hashes[0], "0x2dfe239e");
        assert_eq!(analysis.pak_entries[1].size, 888);
    }

    fn synthetic_pak(entries: &[(u32, &[u8])]) -> Vec<u8> {
        let header_len = (entries.len() + 1) * PAK_HEADER_SIZE;
        let mut data = vec![0; header_len];
        let mut data_offset = header_len;

        for (index, (hash, entry_data)) in entries.iter().enumerate() {
            let header_start = index * PAK_HEADER_SIZE;
            write_u32(&mut data, header_start, 0);
            write_u32(
                &mut data,
                header_start + 4,
                (data_offset - header_start) as u32,
            );
            write_u32(&mut data, header_start + 8, entry_data.len() as u32);
            write_u32(&mut data, header_start + 12, 0);
            write_u32(&mut data, header_start + 16, *hash);
            write_u32(&mut data, header_start + 20, 0x70381cdc);
            write_u32(&mut data, header_start + 24, 0);
            write_u32(&mut data, header_start + 28, 0);
            data.extend_from_slice(entry_data);
            data_offset += entry_data.len();
        }

        write_u32(&mut data, entries.len() * PAK_HEADER_SIZE, PAK_FTYPE_LAST);
        data
    }

    fn analyze_synthetic_pak(pak: &[u8], checksum: &str) -> SongPakAnalysis {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "ghwtdeinitool-song-pak-{}-{unique}.pak.xen",
            std::process::id()
        ));
        fs::write(&path, pak).expect("synthetic pak should be written");
        let analysis = analyze_song_pak(&path.to_string_lossy(), checksum)
            .expect("synthetic pak should analyze");
        let _ = fs::remove_file(path);
        analysis
    }

    fn synthetic_qb(sections: &[(u32, u32, &[u32], bool)]) -> Vec<u8> {
        let header_len = 8;
        let section_headers_len = sections.len() * 20;
        let mut data = vec![0; header_len + section_headers_len];
        let mut child_offset = data.len();

        for (index, (id, child_type, values, dummy_floats)) in sections.iter().enumerate() {
            let section_start = header_len + index * 20;
            write_u32(&mut data, section_start, SECTION_ARRAY);
            write_u32(&mut data, section_start + 4, *id);
            write_u32(&mut data, section_start + 8, 0);
            write_u32(&mut data, section_start + 12, child_offset as u32);
            write_u32(&mut data, section_start + 16, 0);

            data.resize(child_offset + 4, 0);
            write_u32(&mut data, child_offset, *child_type);
            child_offset += 4;

            if *dummy_floats {
                data.resize(child_offset + 8, 0);
                write_u32(&mut data, child_offset, 0);
                write_u32(&mut data, child_offset + 4, 0);
                child_offset += 8;
                continue;
            }

            data.resize(child_offset + 8 + values.len() * 4, 0);
            write_u32(&mut data, child_offset, values.len() as u32);
            write_u32(&mut data, child_offset + 4, 0);
            child_offset += 8;

            for value in *values {
                write_u32(&mut data, child_offset, *value);
                child_offset += 4;
            }
        }

        data
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
}
