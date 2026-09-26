# PDF song input implementation plan

## Goal

Add a Rust PDF input to `chordlib` for chord-and-lyric PDFs that follow the
searchable-text layout shown by `test2.pdf`. Extract the song into the existing
`Song` model so the CLI can convert it to Markdown, ChordPro, JSON, or another
supported output.

The first version targets this specific family of structured PDFs. It does not
attempt to understand arbitrary PDFs or PDFs whose musical text is converted to
vector outlines, such as `test.pdf`.

## Confirmed format facts

- `test2.pdf` is a two-page SongSelect-style chord sheet.
- Its title, artist line, key, tempo, time signature, section headings, lyrics,
  chord symbols, and footer are present as extractable PDF text.
- The PDF's text stream order is not the same as its visual reading order.
  Layout reconstruction must use text positions rather than just concatenate
  extracted strings.
- Chords are placed above their lyric positions. The document also has
  chord-only passages, bar marks, section cues, and a footer with CCLI and
  copyright information.
- In `test.pdf`, the title and some metadata are extractable, but the lyrics and
  chords are vector outlines. That file needs OCR or glyph-shape recognition
  and is outside this initial implementation.

## Scope and compatibility policy

### Version 1 support

- Import searchable-text PDFs matching the page structure and typography of
  `test2.pdf`.
- Support multiple pages and section continuation across page boundaries.
- Extract title, artist names, key, tempo, time signature, copyright, and
  available CCLI metadata.
- Reconstruct section titles, lyric lines, chord placement, chord-only lines,
  instrumental bar marks, and visible annotations.
- Return a contextual parse error when the PDF has no usable text layer, lacks
  required song data, or has ambiguous song content that cannot be mapped
  safely.
- Preserve unsupported but useful metadata as namespaced `Song::tags` where
  practical.

### Explicit non-goals for version 1

- OCR or vector-outline recognition for files like `test.pdf`.
- General-purpose import of arbitrary chord-sheet or scanned PDFs.
- Pixel-perfect reconstruction of PDF layout or typography.
- Guessing when chord-to-lyric alignment is ambiguous.
- A lossless PDF round trip; the output formats represent the extracted song,
  not its original page design.
- Committing `test2.pdf` as a fixture unless its redistribution rights are
  confirmed. Prefer generated, redistributable test PDFs for CI.

## Data mapping

| PDF content | `chordlib` | Notes |
| --- | --- | --- |
| Song title | `Song::titles` | Required for a successful song import. |
| Artist names | `Song::artists` | Split the displayed ` | ` delimiter only if confirmed by fixtures. |
| Key | `Song::key` | Parse through `SimpleChord`; report the field on errors. |
| Tempo | `Song::tempo` | Parse the integer. Preserve an additional beat-unit annotation such as `(1/8)` in a namespaced tag if needed. |
| Time signature | `Song::time` | Parse numerator and denominator, such as `6/8`. |
| Copyright | `Song::copyright` | Prefer the visible copyright line. |
| CCLI and publisher footer fields | `Song::tags` | Keep under a `pdf.` namespace unless the model later gains dedicated fields. |
| Section headings | `Section::title` | Derive from the bold section labels in visual order. |
| Lyric rows and chords | `Section::lines` / `Line::parts` | Attach each chord using its horizontal position above the lyric row. |
| Repeat, transition, and parenthetical cues | comment `Part` or tags | Preserve as text; do not infer repeat counts without an explicit, reliable rule. |
| Instrumental bar marks | lyric text in `Part::languages` | Keep bar separators and chord-only content in their displayed order. |

## Implementation phases

### 1. PDF backend decision

Use [`pdf_oxide`](https://docs.rs/pdf_oxide/latest/pdf_oxide/document/struct.PdfDocument.html)
with default features disabled. It is a pure Rust in-process parser that
provides per-character bounds, origins, and font metadata, which are needed to
rebuild this PDF's visual reading order and chord placement. Its default
optional rendering, OCR, and related features are not needed. The sample's two
pages expose searchable lyric and chord glyphs, including raised chord
extensions, through this API. No Pdfium library or external helper process is
required.

The dependency list pins `office_oxide` at 0.1.9 because `pdf_oxide` 0.3.78
declares a compatible-version range that currently resolves to a newer release
with a breaking `DocumentIR` field change. Keep this transitive pin while that
upstream compatibility issue remains.

### 2. Add a byte-oriented input module

Create `src/inputs/pdf.rs` and expose it from `src/inputs/mod.rs`. Follow the
existing input API pattern:

```rust
pub fn load(path: impl AsRef<std::path::Path>) -> Result<Song, Error>;
pub fn load_bytes(input: &[u8]) -> Result<Song, Error>;
```

Convert PDF library errors into `Error` values with page or operation context.
Keep PDF-specific parsing types private to the module unless later reuse makes
them part of the public API.

### 3. Extract and normalize positioned text

Convert the PDF backend output into owned records containing at least:

- page index;
- text or character;
- bounding box or origin (`x`, `y`);
- font size and available style information.

Normalize page rotation and coordinate direction. Sort and group using
coordinates rather than the PDF's original text order. Prefer per-character
positions where chord superscripts or adjacent chord/lyric elements share a
text segment.

### 4. Parse metadata and page regions

- Identify the title and metadata block near the top of the first page.
- Parse the key, tempo, and time signature with strict numeric validation.
- Map artist and copyright text to `Song` fields.
- Preserve CCLI and publisher details as safe `pdf.` tags.
- Distinguish body content from page headers, logos, and footers so they do not
  become lyric lines.
- Return an actionable error if the page has visible content but no usable text
  layer.

### 5. Rebuild song structure and chord alignment

- Detect section headings by position, font size/style, capitalization, and
  known heading patterns; preserve unfamiliar headings as section names when
  their visual role is clear.
- Process pages in page order and sections in visual order.
- Cluster text by baseline into lyric rows, using a tolerance based on font size
  instead of a fixed page-coordinate threshold.
- Recognize chord tokens with the existing chord parser. Handle explicit
  no-chord labels such as `N.C.` separately if they are not valid `Chord`
  values.
- Merge chord text split across differently sized glyphs (for example, a root
  and superscript extension) before chord parsing.
- Associate each chord with the nearest following lyric position on the same
  row, preserving multiple chords in order.
- Preserve chord-only rows, visible lyric hyphens, instrumental bar separators,
  and parenthetical cues. Do not silently discard text that does not fit the
  expected structure; return an error or preserve it as a comment/tag.
- Map the assembled content into `Song`, `Section`, `Line`, and `Part` without
  changing the core model unless a concrete representation gap is found.

### 6. Wire the CLI and document the supported subset

- Add case-insensitive `.pdf` input dispatch in `src/main.rs`.
- Update the CLI input help text and README with the supported PDF requirements
  and pure Rust runtime behavior.
- Keep the existing output selection so `.md`, `.cp`/`.chopro`, `.json`, and
  other current targets work after import.
- Document that scanned PDFs and outlined-text PDFs are not supported by this
  first version.

### 7. Add tests and verify interoperability

Use generated or redistributable PDFs for committed fixtures. Cover:

1. Metadata extraction and validation, including missing/invalid values.
2. Positioned text ordering when PDF stream order differs from visual order.
3. Chord-to-lyric association, multiple chords per line, slash chords,
   extensions, and superscripts.
4. Section headings, multi-page continuation, chord-only rows, bar marks, and
   annotations.
5. PDFs with no text layer, malformed PDFs, and unsupported/ambiguous layouts.
6. CLI input routing with uppercase/mixed-case `.PDF` extensions and conversion
   to at least one text output.

Manually verify `test2.pdf` locally against the rendered pages, but do not make a
copyrighted sample required for CI. Before the implementation is complete, run
the repository's required checks: `cargo fmt --all -- --check`,
`cargo clippy --all-features -- -D warnings`, `cargo test --verbose`,
`cargo check --all-features`, and `cargo doc --no-deps --all-features`.

## Dependencies and distribution

The selected backend is `pdf_oxide` 0.3.78 with default features disabled. PDF
support is always available and does not require Pdfium, another native
library, OCR, or a helper process. `office_oxide` is pinned to 0.1.9 because
`pdf_oxide` 0.3.78 currently fails to compile against the newer compatible
release 0.1.12; retain that constraint until the upstream dependency range is
corrected.

Run `cargo audit` and configured `cargo deny` checks after changing the PDF
dependencies. The current audit reports `ttf-parser` 0.25.1 as unmaintained
through `pdf_oxide`; it reports no vulnerability for that dependency.

## Definition of done

- `inputs::pdf::load(path)` and `load_bytes(&[u8])` parse the supported
  searchable-text layout into `Song`.
- The CLI accepts `.pdf` input case-insensitively and can convert the result to
  existing output formats.
- Lyrics, chords, sections, supported metadata, and page order are preserved
  for the documented PDF subset.
- Textless or ambiguous PDFs return useful errors instead of partial or
  fabricated songs.
- Documentation states the supported searchable-text subset and the pure Rust
  runtime behavior.
- Formatting, Clippy, tests, type checking, and docs build pass as listed above.

## Known limitations and dependency notes

1. Import relies on character text and positions, so scanned and
   outlined-text PDFs are unsupported.
2. Layout recognition is intentionally limited to the single-column family
   shown in `test2.pdf`; materially different layouts may be rejected as
   ambiguous.
3. The `office_oxide` compatibility pin and `ttf-parser` maintenance warning
   should be revisited when updating `pdf_oxide`.

Do not claim support for outlined-text PDFs or materially different layouts
until they have a separate parser path and fixtures.
