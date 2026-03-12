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

To parse an Ultimate Guitar tab from HTML, first obtain the HTML (for example by saving the page in a browser), read it into a string, and then call `chordlib::inputs::ultimate_guitar::load_html(&html)` from your own code. The library no longer performs live HTTP requests to Ultimate Guitar.

## Feature flags

- `bin`: build the `chordlib` CLI.
- `html`: parse HTML tabs and render HTML output.

## License

[![AGPL-3.0](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
