//! Library helpers to parse, transform, and render chord-and-lyrics songs.
//!
//! The crate focuses on song formats such as ChordPro, SongBeamer, experimental
//! modern ProPresenter `.pro`, and Ultimate Guitar tabs. It provides matching
//! parsers and renderers where the source format permits them. Feature flags:
//! - `html`: parse and render HTML content.
//! - `bin`: build the `chordlib` CLI (depends on `clap`).
//!
//! See the README for end-to-end examples.

mod error;
mod propresenter_proto;
mod propresenter_rtf;
mod unicode_space;

/// Normalization helpers for user-facing text (e.g. Unicode space separators to ASCII).
pub mod text {
    pub use crate::unicode_space::{normalize_space_separators, remove_space_separators};
}

pub use error::Error;

pub mod inputs;
pub mod outputs;
pub mod types;
