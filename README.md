# chordlib

Rust helpers to parse, transform, and render chord-and-lyrics songs. The crate understands common formats (ChordPro, Ultimate Guitar tabs) and can render to HTML, ChordPro, or a terminal-friendly view.

## Installation

```bash
cargo add chordlib
```

To install the CLI, enable the `bin` feature (and optionally `html` for Ultimate Guitar HTML parsing support):

```bash
cargo install chordlib --features "bin html"
```

## Library example

```rust
use chordlib::inputs::chord_pro;
use chordlib::outputs::FormatRender;

fn main() -> Result<(), chordlib::Error> {
    let song = chord_pro::load_string(r#"
{title: Example Song}
{artist: An Artist}

[C]Line one with [G]chords
[F]Line two keeps the [C]beat
"#)?;

    // Render a terminal-friendly output.
    println!("{}", song.format_render(None, None, None));
    Ok(())
}
```

## CLI quick start

Render a ChordPro/ChordPro-like file to HTML from this repository:

```bash
cargo run --features=bin -- path/to/song.wp -o path/to/song.html
```

Rendered HTML will be written to the path given after `-o`.

The CLI supports transposition (`--key`), Nashville notation (`--nashville`), vowel-based chord shifting (`--vowel-move`), and output formats including ChordPro (`.cp`/`.wp`), HTML, and JSON.

ChordPro I/O supports **Nashville number notation** for chords: you can load files that use numbers (e.g. `[1][4][5]`, `[1m][4][5]`, `[b7/2]`) when the key is set with a letter name (e.g. `{key: C}`). Numerals are interpreted as scale degrees relative to that key; letter spellings (e.g. `[C]`, `[F]`) are also supported. The `{key: …}` directive must be a letter name, not a number. Use the `--nashville` flag when writing ChordPro to output chords in Nashville form; the key is always written as a letter (e.g. `{key: C}`).

To parse an Ultimate Guitar tab from HTML, first obtain the HTML (for example by saving the page in a browser), read it into a string, and then call `chordlib::inputs::ultimate_guitar::load_html(&html)` from your own code. The library no longer performs live HTTP requests to Ultimate Guitar.

## Benchmarks

HTML rendering performance benchmarks are available and require the Rust nightly toolchain because they use the unstable `#[bench]` harness:

```bash
rustup install nightly
cargo +nightly bench --bench html_render_bench
```

These benchmarks render real-world Worship Pro / ChordPro-style inputs (for example `song/jesus_lebt3.wp`, which includes a mix of normal and chord-only lines) through the same `FormatHTML` implementation used by the library and CLI. The crate itself remains fully compatible with stable Rust; only the benchmarks require nightly.

## Feature flags

- `bin`: build the `chordlib` CLI.
- `html`: parse HTML tabs and render HTML output.

## License

[![AGPL-3.0](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
