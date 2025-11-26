//! Library helpers to parse, transform, and render chord-and-lyrics songs.
//! 
//! The crate focuses on common song markup formats such as ChordPro and
//! Ultimate Guitar tabs, while providing renderers for HTML and ChordPro
//! outputs. Feature flags:
//! - `download`: enable fetching Ultimate Guitar tabs over the network.
//! - `html`: parse and render HTML content.
//! - `bin`: build the `chordlib` CLI (depends on `clap`).
//! 
//! See the README for end-to-end examples.

mod error;
pub use error::Error;

pub mod inputs;
pub mod outputs;
pub mod types;
