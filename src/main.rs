use clap::Parser;

use chordlib::outputs::{FormatChordPro, FormatRender};
use chordlib::types::SimpleChord;
use chordlib::Error;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input (file or url)
    pub input: String,
    /// A boolean flag if the song should be rendered to the stdout
    #[arg(short, long, default_value_t = false)]
    pub render: bool,
    /// The chordpro output path
    #[arg(short, long, default_value_t = String::default())]
    pub output: String,
    #[arg(short, long)]
    pub key: Option<u8>,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let mut song = if args.input.starts_with("https://tabs.ultimate-guitar.com/") {
        chordlib::inputs::ultimate_guitar::load_url(&args.input)
    } else if args.input.ends_with(".cp") {
        chordlib::inputs::chord_pro::load(&args.input)
    } else {
        Err(Error::Other(format!(
            "unknown input format ({})",
            args.input
        )))
    }?;

    if let Some(key) = args.key {
        song.transpose(SimpleChord::new(key));
    }

    if args.render {
        println!("{}", song.format_render(None, None));
    }

    if args.output.ends_with(".cp") {
        Ok(std::fs::write(
            args.output,
            (&song).format_chord_pro(None, None),
        )?)
    } else if args.output.ends_with(".json") {
        Ok(std::fs::write(args.output, serde_json::to_string(&song)?)?)
    } else if args.output.len() == 0 {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "unknown output format ({})",
            args.output
        )))
    }?;
    Ok(())
}
