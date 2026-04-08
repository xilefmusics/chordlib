# Changelog

All notable changes to this project are documented in this file.

## [0.7.0] — HTML bars, chord parsing & layout

### ✨ New features

- **🎸 HTML 6/8 chord-only bars** — Compound-beat and two-eighth layouts for 6/8 time; continuation uses `/` or `·` when the second beat splits. (#37)
- **🎸 HTML full-bar chords** — When one harmonic segment spans the full bar, render only the chord symbol (no per-beat `/` or `·` fillers). (#39)

### 🐛 Fixes

- **Chord duration in meter** — Interpret `:n` chord duration in meter beats consistently in layout. (#36)
- **Locale decimals** — Accept comma as the decimal separator in chord duration tokens (normalized to a dot for parsing). (#35)
- **Slash bass spelling** — Derive default slash-bass letter names from the root, semitone interval to the bass, and enharmonic tables (parsing still accepts `B#`, `E#`, `Cb`, `Fb`). (#34)

### 🛠️ Other

- **GitHub** — Bug and feature issue templates with structured fields. (#33)

---

## [0.6.0] — Repeat, Nashville & tags

Lots of improvements since 0.5.0: ChordPro repeat directives, Nashville number notation, custom metadata tags, multi-language content, better HTML bar rendering, and Worship Pro timing fixes. One breaking change: Ultimate Guitar is no longer fetched over HTTP by the crate (see migration below).

### ✨ New features

- **🔄 ChordPro repeat directives** — Parse and emit `{repeat}` and `{repeat: N}`; section repeat counts are preserved and shown in HTML. (#17)
- **🎹 Nashville number notation** — ChordPro I/O supports Nashville chords (e.g. `[1][4][5]`) when key is a letter name; use `--nashville` when writing. (#18)
- **🏷️ Custom tags** — Songs have a `tags` map; ChordPro `{meta: name value}` is read/written for filtering and finding songs. (#28)
- **🌍 Multi-language song content** — Support for multiple titles, artists, and languages; ChordPro and Worship Pro I/O handle multi-language lines. (#24)
- **🥁 Beat slashes in HTML** — Chord-only lines are rendered as bars with one symbol per beat (chord or `/`). (#19)
- **⏱️ Worship Pro timing** — Chord duration as milliclicks and decimal clicks in the format. (#16)

### 🐛 Fixes

- **Chord duration in compound meter** — `:n` on chords is interpreted as **notated beats** (denominator of the time signature), so in 6/8 a `:1` chord spans one eighth-note column in HTML/SVG bar grids; 4/4 behavior is unchanged. (#30)
- **Ultimate Guitar import** — First section is no longer dropped when it has a section title (e.g. `[Verse 1]`). (#20)
- **Worship Pro export** — Repeat directives are preserved on export. (#23)
- **ChordPro import** — CCLI repeat markers `[||:]` and `[:||]` are stripped so import succeeds. (#6)

### 🛠️ Other

- **CI** — GitHub Actions workflow with `cargo fmt`, `cargo clippy`, `cargo test`. (#27)
- **Code quality** — Refactor, dependency updates, Clippy clean, and HTML rendering benchmarks. (#25)
- **Docs** — README improvements.
- **Chore** — `.DS_Store` removed from tracking. (#4)

### ⚠️ Breaking change & migration

**Ultimate Guitar HTTP fetching removed**

The crate no longer fetches Ultimate Guitar pages. The `download` feature and `reqwest` are gone. The CLI only accepts ChordPro (or ChordPro-like) files.

**What to do:** If you need Ultimate Guitar content, fetch or save the HTML yourself and call:

```rust
chordlib::inputs::ultimate_guitar::load_html(&html)
```

Parsing behavior is unchanged; only the way you obtain the HTML changes.

### 📦 JSON / database migration (songs stored as JSON)

The internal song model is used by some users for storage (e.g. JSON in a DB). Schema changes in 0.6.0 are **backward-compatible for reading**:

- **Song:** New optional fields `titles`, `artists`, `languages`, and new `tags` (object). When these are missing, 0.6.0 deserializes as `None` or empty `tags`. No change required to existing JSON for 0.6.0 to load it.
- **Section:** New field `repeat_count` (default `1`). Missing field is treated as 1.
- **Part:** Unchanged (`languages` array as before).

**If you store song JSON in a database:**

- **Option A — No migration:** Keep existing documents as-is. 0.6.0 will read them correctly. New or updated songs may include `titles`, `artists`, `languages`, `tags`, and `repeat_count`.
- **Option B — Normalize stored documents:** Run a one-off migration that adds default values so every document has the new shape (e.g. add `"tags": {}` where missing, `"repeat_count": 1` on sections).
- **Option C — Schema validation:** If you validate with a JSON Schema (or similar), update it to allow the new optional Song fields and the new Section field so 0.6.0 output is accepted.

**Summary:** No mandatory migration for reading; optional migration only if you want a uniform schema or stricter validation.

---

## [0.5.0] and earlier

See git history or tags for earlier releases.

[0.7.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.7.0
[0.6.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.6.0
[0.5.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.5.0
