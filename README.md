# chordlib

Rust helpers to parse, transform, and render chord-and-lyrics songs. The crate understands common formats (ChordPro, Ultimate Guitar tabs) and can render to HTML, ChordPro, or a terminal-friendly view.

## Installation

```bash
cargo add chordlib
```

To install the CLI, enable the `bin` feature (and optionally `download`/`html`):

```bash
cargo install chordlib --features "bin download html"
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

Render an Ultimate Guitar tab to HTML:

```bash
chordlib https://tabs.ultimate-guitar.com/tab/artist/song-123456 \
  --render \
  --output song.html
```

The CLI supports transposition (`--key`), Nashville notation (`--nashville`), vowel-based chord shifting (`--vowel-move`), and output formats including ChordPro (`.cp`/`.wp`), HTML, and JSON.

## Feature flags

- `bin`: build the `chordlib` CLI.
- `download`: fetch Ultimate Guitar tabs over HTTP.
- `html`: parse HTML tabs and render HTML output.

## License

[![GPL-3.0](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
