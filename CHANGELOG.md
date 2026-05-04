# Changelog

All notable changes to this project are documented in this file.

## [0.9.0] — Default spelling matrix & keyed ChordPro parsing

### Breaking changes

- **Keyed ChordPro** — With `{key: …}`, chord roots must use **letter spellings** (e.g. `[C]`). Nashville numerals inside brackets (`[1]`, `[b7]`, …) are no longer interpreted as scale degrees. Nashville **output** (`ChordRepresentation::Nashville`) from letter chords is unchanged (#60).
- **`Chord` serde** — Removed field `main_is_nashville_numeral`; unknown JSON keys remain ignored by default deserialization (#60).

### 🛠️ Refactor

- **Default chord spelling** — Per-key symbol tables, `ChordRepresentation::symbols`, and slash-bass spelling aligned with the chord-root row; `SimpleChord::format` simplified; ChordPro tests and HTML benchmarks updated (#60).

---

## [0.8.3] — Nashville b7 spelling

### 🐛 Fixes

- **Nashville `b7` (default letters)** — Keyed numerals such as `b7` now print the diatonic lowered seventh (e.g. `Bb` in `C` instead of `A#`). Letter chords and ChordPro transposition behavior are unchanged (#58, #59).

### 🛠️ Other

- **`Chord` serde** — Optional `main_is_nashville_numeral` (default `false`) records whether the root came from the Nashville parse path with a song key; it is cleared on `normalize`.

---

## [0.8.2] — ChordPro pipe bars, HTML grid & chord spacing

### 🐛 Fixes

- **ChordPro pipe bars** — Empty bracket groups inside `|` bar lines are ignored; measures that use `|` are split so each bar matches a full notated measure (#56, #57).
- **ChordPro `{copyright}`** — Directive is emitted with the correct spelling (#52).
- **HTML chord-only bars** — Bar grid aligns when consecutive chord-only lines have different lengths (#54).
- **Chord text** — Unicode space separators are normalized and stripped consistently when parsing chord symbols (#55).

---

## [0.8.1] — Nashville degrees & key-aware ChordPro

### 🐛 Fixes

- **Nashville scale degrees** — Nashville output no longer double-counts the song key; chord roots use the correct degree relative to the key (#50, fixes #49).
- **ChordPro + key** — With a resolvable `{key: …}`, chord roots are parsed key-relative so Nashville numerals and letter chords align with the key; documents absolute vs key-relative conventions on `Chord` and `SimpleChord::normalize` (#50).

---

## [0.8.0] — Song vectors, chord spelling & HTML bars

### ✨ New features

- **🎸 HTML equal-length chord segments** — When every chord segment in a full bar has the same duration, render chord names only (no per-beat `/` or `·` continuation tokens). Partial bars and unequal lengths keep the beat grid. (#47)

### 🐛 Fixes

- **Chord spelling (slash bass)** — `Chord::format` now emits extensions before the slash bass (e.g. `C4/E` instead of the incorrect `C/E4`). (#44)
- **HTML bar continuation** — When explicit chord segments end before the bar duration, remaining beat cells continue the last harmony with `/` instead of `·`. (#45)

### ⚠️ Breaking change & migration

**`Song` uses vectors only for titles, artists, and languages (#46)**

Scalar `title` / `artist` / `language` and `Option<Vec<…>>` wrappers are removed. Primary values come from index 0 via `title()`, `artist()`, and `language()` (empty string when the vector is empty). ChordPro parsing and Worship Pro I/O use the vectors directly; serde defaults missing fields to empty `Vec`s.

**What to do:** Update downstream code to the new public `Song` fields and getters. If you deserialize stored JSON, align payloads with the new shape or rely on serde defaults where applicable.

---

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

[0.9.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.9.0
[0.8.3]: https://github.com/xilefmusics/chordlib/releases/tag/0.8.3
[0.8.2]: https://github.com/xilefmusics/chordlib/releases/tag/0.8.2
[0.8.1]: https://github.com/xilefmusics/chordlib/releases/tag/0.8.1
[0.8.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.8.0
[0.7.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.7.0
[0.6.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.6.0
[0.5.0]: https://github.com/xilefmusics/chordlib/releases/tag/0.5.0
