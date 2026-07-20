# SongBeamer input/output implementation plan

## Goal

Add first-class SongBeamer `.sng` import and export to the `chordlib` library and
CLI. Preserve the song information represented by the current `Song` model,
including multilingual lyrics, section/verse order, chords, key, tempo, time
signature, title, author, and copyright.

The initial target is SongBeamer's text-based `#Version=3` format. The work
should be driven by real, redistributable fixtures saved by SongBeamer rather
than by assumptions about its undocumented fields.

## Confirmed format facts

- Header fields use `#Name=value`; the lyric body follows a `---` separator.
- Slides are separated by `---` (with `--` and `--A` having related but
  different presentation semantics).
- `#VerseOrder` is a comma-separated sequence of verse markers and may refer to
  the same marker multiple times without duplicating its body.
- A slide's first line can be a standard marker such as `Vers 1`, `Chorus`, or
  `Bridge`, or a custom `$$M=...` marker.
- With multiple languages, lyrics are stored in repeating groups of lines;
  `#LangCount` determines the group size. SongBeamer also has explicit line
  prefixes such as `##1` and `##2`, which need fixture-based coverage.
- `#Chords` is base64-encoded and contains chord positions tied to the lyric
  body. Editing slide or marker lines can shift those positions.
- SongBeamer detects ANSI/Windows-1252, UTF-8, UTF-16LE, and UTF-16BE by BOM;
  without a BOM it assumes Windows-1252. New files should be UTF-8 with BOM.

## Scope and compatibility policy

### Version 1 support

- Read `.sng` files using Windows-1252, UTF-8, UTF-16LE, or UTF-16BE according
  to SongBeamer's BOM rules.
- Write deterministic `#Version=3` files as UTF-8 with BOM and CRLF endings.
- Map all fields that have an equivalent in `Song` and preserve safe,
  unsupported scalar header fields in `Song::tags` under a `songbeamer.`
  namespace.
- Import and export normal slide separators, standard/custom verse markers,
  `#VerseOrder`, multilingual lyric groups, and the `#Chords` payload.
- Recognize `--` and `--A` on input, documenting that both collapse to a normal
  `Section` boundary because the current model does not represent their display
  differences.
- Ignore visual-only properties (font, colors, backgrounds, transitions, and
  editor identity) semantically. Preserve safe scalar values through tags when
  practical, but do not add presentation fields to `Song` in this change.

### Explicit non-goals for version 1

- Pixel-perfect preservation of SongBeamer presentation settings.
- Melody/audio/background asset import.
- Byte-for-byte round trips of arbitrary `.sng` files.
- Lossless representation of SongBeamer features for which `Song` has no model,
  including the distinct display behavior of `--` and `--A`.
- Guessing malformed chord offsets. Invalid base64, records, or out-of-range
  positions should return a contextual parse error.

## Data mapping

| SongBeamer | `chordlib` | Notes |
| --- | --- | --- |
| `#Title`, `#TitleLangN` | `Song::titles` | Preserve language index and empty slots. |
| `#Author` | `Song::artists` | Confirm delimiter behavior with fixtures; do not split names speculatively. |
| `#(c)` / copyright fields | `Song::copyright` | Define precedence when several copyright-like fields occur. |
| `#Key` | `Song::key` | Reuse `SimpleChord`; add fixtures for German `H`/`B`/`B=` spellings. |
| `#Tempo` | `Song::tempo` | Reject invalid numbers with field context. |
| `#Time` | `Song::time` | Accept the exact forms emitted by SongBeamer fixtures. |
| `#LangCount` | `Part::languages` | Reconstruct each logical lyric line across all languages. |
| slide marker | `Section::title` | Normalize recognized markers only enough for flow matching; preserve custom text. |
| slide body | `Section::lines` | Convert lyric/chord positions into ordered `Part` values. |
| `#VerseOrder` | section order/reference form | Parse unique bodies first, then use `Song::apply_flow`; repeated references should become empty-body sections as supported by the existing model. |
| `#Chords` | `Part::chord` | Decode positions after the exact body/line-ending rules are established. |
| other safe scalar headers | `Song::tags` | Store as `songbeamer.<name>` to avoid collisions. |

For export, use `Song::flow_items()` and `Song::custom_flow()` to emit one body
per distinct section variant plus a `#VerseOrder` that reproduces the expanded
order and repeat counts. Define a deterministic marker for untitled sections
and ensure duplicate titles with different bodies use unique marker names.

## Implementation phases

### 1. Build a compatibility fixture corpus

Add small, original/public-domain files under `tests/fixtures/songbeamer/` for:

- each supported encoding and BOM combination, including BOM-less CP1252;
- LF and CRLF input;
- minimal metadata and unknown header fields;
- one, two, and three languages, including Unicode text;
- standard, numbered, custom, missing, and repeated verse markers;
- `#VerseOrder` with repeated markers and a section repeat count;
- `---`, `--`, and `--A` separators;
- chords at line start, mid-word, end-of-line, chord-only lines, adjacent chords,
  slash chords, German notation, and non-ASCII lyric offsets;
- malformed header values, malformed base64, invalid chord records, and chord
  positions outside the lyric body.

For every chord fixture, save the source `.sng` in SongBeamer and record the
decoded `#Chords` payload in a neighboring test note. This makes the offset
rules auditable.

### 2. Add encoding and low-level document parsing

Create `src/inputs/songbeamer/` with small, separately tested components:

- `decode.rs`: BOM detection and decoding to Rust `String`; use Windows-1252
  only when no BOM is present;
- `document.rs`: split headers from body without changing body offsets, retain
  line-ending and separator information long enough to decode chords;
- `metadata.rs`: typed parsing and validation of supported headers;
- `chords.rs`: base64 decoding and parsing of chord records into an intermediate
  position table;
- `mod.rs`: map the intermediate document into `Song`.

Expose:

```rust
pub fn load(path: impl AsRef<std::path::Path>) -> Result<Song, Error>;
pub fn load_bytes(input: &[u8]) -> Result<Song, Error>;
pub fn load_string(input: &str) -> Result<Song, Error>;
```

`load_string` is the convenient already-decoded UTF-8 entry point;
`load_bytes` is the canonical parser for actual `.sng` files. Include byte,
line, field, or chord-record context in parse errors where possible.

### 3. Map slides, languages, and flow into `Song`

- Parse physical slides without prematurely discarding marker lines, because
  chord positions may count them.
- Detect standard marker lines and `$$M=` custom markers.
- Reconstruct multilingual logical lines from `#LangCount` and explicit
  language prefixes. Reject incomplete/ambiguous groups unless fixtures show a
  well-defined SongBeamer fallback.
- Convert each lyric line plus decoded chord positions to `Line`/`Part` while
  preserving Unicode boundaries. Be explicit whether SongBeamer offsets count
  bytes, UTF-16 code units, Unicode scalar values, or half-character units.
- Parse distinct sections, resolve `#VerseOrder` to `SongFlowItem`s, and call
  `Song::apply_flow`. Produce an error for unknown or ambiguous markers instead
  of silently choosing a section.
- Normalize absolute chords once, following the same invariant used by the
  ChordPro input.

### 4. Add deterministic SongBeamer output

Create `src/outputs/songbeamer/` and export a `FormatSongBeamer` trait from
`src/outputs/mod.rs`. The renderer should:

- serialize a canonical header order and `#Version=3`;
- write `#LangCount`, localized titles, mapped metadata, safe preserved fields,
  unique section bodies, and `#VerseOrder`;
- render each language group and verse marker into an intermediate body first;
- calculate chord positions from that exact body representation, then encode
  the `#Chords` header;
- format chords through the existing chord representation APIs, including
  transposition and German spelling rules confirmed by fixtures;
- return bytes with UTF-8 BOM and CRLF line endings.

Suggested public API:

```rust
pub trait FormatSongBeamer {
    fn format_songbeamer(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
    ) -> Result<Vec<u8>, Error>;
}
```

Do not offer a single-language filter initially: changing the language count
also changes the chord/body offsets and would silently discard data. It can be
added later as an explicit export option.

### 5. Wire the CLI and public documentation

- Add `pub mod songbeamer;` to `src/inputs/mod.rs` and export
  `FormatSongBeamer` from `src/outputs/mod.rs`.
- In `src/main.rs`, recognize `.sng` input case-insensitively and load it as
  bytes. Recognize `.sng` output and write the renderer's bytes.
- Replace the growing extension `if` chain with a small parsed input/output
  format enum so detection and error messages can be unit tested.
- Update CLI help, crate-level docs, and `README.md` with the supported subset,
  encoding behavior, a library example, and a CLI conversion example.
- Update `CHANGELOG.md` when the feature is released.

### 6. Test interoperability and round trips

Add tests at three levels:

1. Unit tests for encoding, metadata, separator, verse marker, verse order, and
   chord-record codecs.
2. Fixture tests asserting `.sng -> Song` and `Song -> .sng -> Song`, comparing
   semantic `Song` values rather than header ordering or original bytes.
3. CLI integration tests for `.sng` input/output, unknown formats, mixed-case
   extensions, transposition, and invalid files.

Finally, open generated fixtures in a supported SongBeamer release and verify:

- titles, authors, key, tempo, and time signature;
- slide grouping and verse order;
- all language lines;
- chord spelling and visual placement, especially around Unicode characters;
- a save/reopen cycle in SongBeamer followed by re-import into `chordlib`.

Record the tested SongBeamer version and any known deviations in the README.

## Dependencies

Likely additions are:

- `base64` for `#Chords`;
- `encoding_rs` for deterministic Windows-1252 decoding (UTF-16 can be handled
  directly or consistently through the same layer).

Before adding them, confirm the standard library cannot meet the required
behavior clearly and safely. If dependencies change, run `cargo audit` and
`cargo deny` when available, in addition to the normal repository gates.

## Definition of done

- Public byte-oriented input and output APIs are documented and exported.
- `.sng` works as both CLI input and output without changing existing formats.
- Supported metadata, lyrics, languages, flow, and chords survive semantic
  round trips.
- Malformed input returns `Error` and never panics.
- Unsupported/lossy fields and separator behavior are documented.
- Generated UTF-8 BOM/CRLF files pass manual interoperability testing in
  SongBeamer.
- `cargo fmt --all -- --check` passes.
- `cargo check --all-features` passes.
- `cargo test` passes.
- `cargo clippy --all-features -- -D warnings` passes with zero warnings.
- `cargo doc --no-deps --all-features` passes.
- If dependencies changed, `cargo audit` and configured `cargo deny` checks are
  run and their results documented.

## Risks and decisions to settle during the fixture phase

1. Exact `#Chords` record grammar and whether offsets count bytes, characters,
   UTF-16 units, or half-character positions.
2. Whether marker and separator lines contribute to chord offsets in every
   supported SongBeamer version.
3. Exact mapping of chord spelling (`H`, `B`, `B=`) to `SimpleChord` and back.
4. `#VerseOrder` escaping when custom marker names contain commas.
5. How SongBeamer represents incomplete multilingual groups and explicit
   `##N` continuation lines.
6. Which unknown header fields are safe scalar values versus encoded/binary or
   path-dependent data that must be dropped.
7. Whether `Song::tags` preservation is sufficient or a dedicated extension
   map is needed for lossless metadata round trips.

Do not finalize the chord codec or claim lossless compatibility until these
questions are answered by fixtures generated in SongBeamer.

## References

- [SongBeamer wiki: Song editor, slide separators, languages, and verse markers](https://wiki.songbeamer.de/index.php?title=Song)
- [SongBeamer forum: encoding and BOM behavior](https://forum-en.songbeamer.de/viewtopic.php?t=166)
- [SongBeamer forum: `#VerseOrder` example and chord storage discussion](https://forum.songbeamer.de/viewtopic.php?t=4657)
- [SongBeamer forum: recommendation to create UTF-8 files with a BOM](https://forum.songbeamer.de/viewtopic.php?t=4574)
- [OpenLP SongBeamer chord-position interoperability report](https://discuss.openlp.org/d/4008-songbeamer-import-chords-start-with-second-line)
- [Independent SongBeamer parser behavior summary](https://www.skypack.dev/view/songbeamer)
