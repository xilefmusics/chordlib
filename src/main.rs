use clap::Parser;

use chordlib::Error;
use chordlib::outputs::{FormatChordPro, FormatHTML, FormatRender};
use chordlib::types::{ChordRepresentation, SimpleChord};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input song file (ChordPro, Worship Pro, or Ultimate Guitar HTML)
    pub input: String,
    /// A boolean flag if the song should be rendered to the stdout
    #[arg(short, long, default_value_t = false)]
    pub render: bool,
    /// The chordpro output path
    #[arg(short, long, default_value_t = String::default())]
    pub output: String,
    #[arg(short, long)]
    pub key: Option<u8>,
    #[arg(short, long, default_value_t = false)]
    pub vowel_move: bool,
    #[arg(short, long, default_value_t = false)]
    pub spacing_remove: bool,
    #[arg(short, long, default_value_t = false)]
    pub nashville: bool,
    /// Language index (0 = default, 1 = second language, ...)
    #[arg(short = 'l', long)]
    pub language: Option<usize>,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let representation = Some(if args.nashville {
        ChordRepresentation::Nashville
    } else {
        ChordRepresentation::Default
    });

    let mut song = if args.input.ends_with(".cp")
        || args.input.ends_with(".wp")
        || args.input.ends_with(".chopro")
    {
        chordlib::inputs::chord_pro::load(&args.input)
    } else if args.input.ends_with(".html") {
        let html = std::fs::read_to_string(&args.input)?;
        chordlib::inputs::ultimate_guitar::load_html(&html)
    } else {
        Err(Error::Other(format!(
            "unknown input format ({})",
            args.input
        )))
    }?;

    if let Some(key) = args.key {
        song.transpose(SimpleChord::new(key));
    }

    if args.vowel_move {
        song = song.move_chords_to_next_vowels();
    }

    if args.spacing_remove {
        song = song.remove_manual_spacing();
    }

    if args.render {
        println!(
            "{}",
            song.format_render(None, representation.as_ref(), args.language)
        );
    }

    if args.output.ends_with(".cp") || args.output.ends_with(".chopro") {
        Ok(std::fs::write(
            args.output,
            (&song).format_chord_pro(None, representation.as_ref(), args.language, false),
        )?)
    } else if args.output.ends_with(".wp") {
        Ok(std::fs::write(
            args.output,
            (&song).format_chord_pro(None, representation.as_ref(), args.language, true),
        )?)
    } else if args.output.ends_with(".json") {
        Ok(std::fs::write(args.output, serde_json::to_string(&song)?)?)
    } else if args.output.ends_with(".html") {
        Ok(std::fs::write(
            args.output,
            (&song).format_html(None, representation.as_ref(), args.language, None),
        )?)
    } else if args.output.is_empty() {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "unknown output format ({})",
            args.output
        )))
    }?;
    Ok(())
}
