# ProPresenter wire fixtures

`current-schema.pro.b64` is a base64-wrapped protobuf `.pro` fixture. It was
assembled directly from the pinned schema's field numbers, independently of
chordlib's exporter. Wrapping keeps the binary reviewable and portable through
text-only source patches; tests decode it before calling `load_bytes`.

The fixture covers CCLI metadata, Unicode/code-page RTF, two slides in one cue
group, a selected arrangement with a repeated group, an empty media cue, a
disabled lyric cue, and a native chord attribute. Other focused cases are
constructed next to the importer tests so failures identify the exact semantic
being exercised.
