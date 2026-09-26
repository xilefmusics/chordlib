use clap::Parser;

use chordlib::Error;
use chordlib::outputs::{
    FormatChordPro, FormatHTML, FormatHTMLWithCapo, FormatMarkdown, FormatProPresenter,
    FormatRender, FormatSongBeamer,
};
use chordlib::types::{ChordRepresentation, SimpleChord};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input song file (ChordPro, Markdown, PDF, SongBeamer, ProPresenter, or Ultimate Guitar HTML)
    pub input: String,
    /// A boolean flag if the song should be rendered to the stdout
    #[arg(short, long, default_value_t = false)]
    pub render: bool,
    /// Output path; the extension selects the output format
    #[arg(short, long, default_value_t = String::default())]
    pub output: String,
    #[arg(short, long)]
    pub key: Option<u8>,
    /// Set a capo fret; shift key metadata while preserving chord shapes
    #[arg(long)]
    pub capo: Option<u8>,
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

    let input_lower = args.input.to_ascii_lowercase();
    let mut song = if input_lower.ends_with(".cp")
        || input_lower.ends_with(".wp")
        || input_lower.ends_with(".chopro")
    {
        chordlib::inputs::chord_pro::load(&args.input)
    } else if input_lower.ends_with(".sng") {
        chordlib::inputs::songbeamer::load(&args.input)
    } else if input_lower.ends_with(".pro") {
        chordlib::inputs::propresenter::load(&args.input)
    } else if input_lower.ends_with(".html") {
        #[cfg(feature = "html")]
        let result = {
            let html = std::fs::read_to_string(&args.input)?;
            chordlib::inputs::ultimate_guitar::load_html(&html)
        };
        #[cfg(not(feature = "html"))]
        let result = Err(Error::Other(
            "Ultimate Guitar HTML input requires the `html` feature".into(),
        ));
        result
    } else if input_lower.ends_with(".pdf") {
        chordlib::inputs::pdf::load(&args.input)
    } else if input_lower.ends_with(".md") || input_lower.ends_with(".markdown") {
        chordlib::inputs::markdown::load(&args.input)
    } else {
        Err(Error::Other(format!(
            "unknown input format ({})",
            args.input
        )))
    }?;

    if let Some(key) = args.key {
        song.apply_key(SimpleChord::new(key));
    }

    let base_key = song.key.as_ref().unwrap_or(&SimpleChord::default()).clone();
    if args.vowel_move {
        song = song.move_chords_to_next_vowels();
    }

    if args.spacing_remove {
        song = song.remove_manual_spacing();
    }

    let html_song = song.clone();
    if let Some(capo) = args.capo {
        apply_capo(&mut song, &base_key, capo);
    }

    if args.render {
        println!(
            "{}",
            song.format_render(None, representation.as_ref(), args.language)?
        );
    }

    let output_lower = args.output.to_ascii_lowercase();
    if output_lower.ends_with(".cp") || output_lower.ends_with(".chopro") {
        Ok(std::fs::write(
            args.output,
            (&song).format_chord_pro(None, representation.as_ref(), args.language, false),
        )?)
    } else if output_lower.ends_with(".wp") {
        Ok(std::fs::write(
            args.output,
            (&song).format_chord_pro(None, representation.as_ref(), args.language, true),
        )?)
    } else if output_lower.ends_with(".json") {
        Ok(std::fs::write(args.output, serde_json::to_string(&song)?)?)
    } else if output_lower.ends_with(".html") {
        let html = match args.capo {
            Some(capo) => (&html_song).format_html_with_capo(
                Some(&base_key),
                representation.as_ref(),
                args.language,
                None,
                capo,
            )?,
            None => (&song).format_html(None, representation.as_ref(), args.language, None)?,
        };
        Ok(std::fs::write(args.output, html)?)
    } else if output_lower.ends_with(".sng") {
        Ok(std::fs::write(
            args.output,
            (&song).format_songbeamer(None, representation.as_ref())?,
        )?)
    } else if output_lower.ends_with(".pro") {
        Ok(std::fs::write(
            args.output,
            (&song).format_propresenter(None, representation.as_ref(), args.language)?,
        )?)
    } else if output_lower.ends_with(".md") || output_lower.ends_with(".markdown") {
        Ok(std::fs::write(
            args.output,
            (&song).format_markdown(None, representation.as_ref())?,
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

fn apply_capo(song: &mut chordlib::types::Song, base_key: &SimpleChord, capo: u8) {
    let capo = capo % 12;
    let chord_shift = (12 - capo) % 12;

    if chord_shift != 0 {
        for section in &mut song.sections {
            for line in &mut section.lines {
                for part in &mut line.parts {
                    part.chord = part.chord.take().map(|chord| chord.transpose(chord_shift));
                }
            }
        }
    }

    song.apply_key(base_key.transpose(capo));
}
