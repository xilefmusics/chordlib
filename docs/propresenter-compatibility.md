# ProPresenter `.pro` compatibility

ProPresenter support is experimental. Chordlib reads and writes standalone,
modern protobuf-based `.pro` presentation files. Generated files have not yet
been opened, saved, and re-imported with a named ProPresenter release, so this
document does not claim production interoperability.

## Schema provenance

The wire model is a focused subset of
[greyshirtguy/ProPresenter7-Proto](https://github.com/greyshirtguy/ProPresenter7-Proto),
pinned to commit `1b63dda196eb7e079721a8a4a7e7773520cb5ad2`. The relevant
MIT license and a minimized schema description are checked into
`vendor/propresenter/`. Rust protobuf definitions are also checked in; building
chordlib does not require `protoc`, network access, or ProPresenter.

Unknown protobuf fields are ignored during import. The public API exposes only
chordlib's song model, not the presentation wire types.

## Import behavior

- `inputs::propresenter::load` reads a file, and `load_bytes` reads an in-memory
  protobuf message. There is deliberately no string loader for this binary
  format.
- Presentation and CCLI metadata map to native `Song` fields where possible.
  Values without a native field use exact `propresenter.*` tags.
- The current/user music key is preferred, followed by original and legacy key
  fields. A populated invalid key is a contextual parse error.
- A selected arrangement determines cue-group order. If none is selected,
  document order is used. Repeated group UUIDs become section references, so
  the song flow retains repeats and same-title variants.
- Enabled presentation cues in a group are combined in cue order. A blank lyric
  line marks an original slide boundary. Media-only and empty groups are
  skipped.
- The first non-empty primary text element supplies each cue's lyrics. Its RTF
  supports nested groups, escaped characters, Unicode controls, code-page
  bytes, and paragraph/line breaks while discarding styling destinations.
- Native `Graphics.Text.ChordPro` ranges are read when present. Files without
  them import as ordinary lyric-only songs. ProPresenter does not reliably
  identify parallel languages, so import uses one chordlib language.

Malformed protobuf, RTF, chord ranges, keys, arrangement references, and empty
presentations return contextual errors rather than panicking.

## Export behavior

`FormatProPresenter::format_propresenter` exports the selected language (index
zero by default). It emits one 1920×1080 cue and slide for each distinct section
body and a selected arrangement matching `Song::custom_flow()`, including
repeated sections and distinct bodies with the same title.

Audience text is canonical lyric-only RTF. Chords are attached separately as
native text ranges for compatible stage layouts. Default letter notation and
Nashville notation are supported, including transposition through the method's
key argument. Native chord payload generation remains experimental until a
generated file has been exercised in ProPresenter.

Slides use centered white text and a transparent/default background. UUIDs are
derived from song content and export options, and volatile timestamps are
omitted, so the same song and options produce identical bytes.

## Known losses and exclusions

Import flattens multiple slides in one cue group into a section, so original
slide boundaries cannot round-trip. Secondary text boxes, formatting, themes,
layouts, transitions, stage layouts, media, timelines, disabled cues, tempo,
time signature, and multilingual structure are not preserved.

Legacy ProPresenter 4–6 XML formats, `.proBundle` packages, playlists, and
library databases are outside the supported scope.
