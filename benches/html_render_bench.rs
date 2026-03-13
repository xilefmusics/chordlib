#![feature(test)]

extern crate test;

use test::{black_box, Bencher};

use chordlib::inputs::chord_pro;
use chordlib::outputs::FormatHTML;
use chordlib::types::{ChordRepresentation, Song};

fn load_song(input: &str) -> Song {
    chord_pro::load_string(input).expect("parse benchmark song")
}

// 1. Mixed lyrics + chords, typical verse/chorus structure.
fn song_mixed_standard() -> Song {
    let mut input = String::from("{title: Mixed Standard}\n{key: C}\n");
    for _ in 0..8 {
        input.push_str(
            "{section: Verse}\n\
             [C]Line one with [G]chords\n\
             [Am]Line two keeps the [F]groove\n\
             [C]Line three keeps it [G]moving\n\
             [F]Line four returns to [C]home\n\
             {section: Chorus}\n\
             [C]Sing it [G]loud, sing it [Am]clear\n\
             [F]Everybody [C]gathers [G]here\n",
        );
    }
    load_song(&input)
}

// 2. Many short sections to stress header/title rendering and section wrappers.
fn song_many_sections() -> Song {
    let mut input = String::from("{title: Many Sections}\n{key: D}\n");
    for i in 1..=60 {
        input.push_str(&format!("{{section: S{}}}\n[D]Line in section {}\n", i, i));
    }
    load_song(&input)
}

// 3. Long lines with dense chords to stress inline layout.
fn song_long_lines_dense_chords() -> Song {
    let mut input =
        String::from("{title: Long Lines Dense Chords}\n{key: G}\n{section: Verse}\n");
    for _ in 0..20 {
        input.push_str(
            "[G]Word [D]word [Em]word [C]word [G]word [D]word [Em]word [C]word\n\
             [G]Another [D]very [Em]busy [C]line [G]with [D]many [Em]chords [C]spread\n",
        );
    }
    load_song(&input)
}

// 4. Chord-only progression with repeat to stress render_bars and repeat marker.
fn song_chord_only_progression() -> Song {
    let mut input = String::from(
        "{title: Chord Only Progression}\n{key: C}\n{section: Progression}\n",
    );
    for _ in 0..200 {
        input.push_str("[C][G][Am][F]\n");
    }
    input.push_str("{repeat}\n");
    load_song(&input)
}

// 5. Multilingual lyrics using &-lines to stress language handling.
fn song_multilingual() -> Song {
    let mut input = String::from(
        "{title: Multilingual}\n{key: C}\n{language: de}\n{language2: en}\n",
    );
    for _ in 0..20 {
        input.push_str(
            "{section: Verse}\n\
             [C]Hallo [G]Welt\n\
             &[C]Hello [G]World\n\
             [F]Noch [C]eine [G]Zeile\n\
             &[F]One [C]more [G]line\n",
        );
    }
    load_song(&input)
}

// 6. Many comments and comment-only lines.
fn song_comment_heavy() -> Song {
    let mut input = String::from("{title: Comment Heavy}\n{key: A}\n");
    for _ in 0..15 {
        input.push_str(
            "{section: Intro}\n\
             {comment: Play freely}\n\
             [A]Line with [E]chords\n\
             {comment: Build up}\n\
             [F#m]Soft [D]section\n\
             {comment: Big chorus}\n\
             {section: Chorus}\n\
             [A]Sing [E]loud\n\
             {comment: Repeat and fade}\n",
        );
    }
    load_song(&input)
}

// 7. Tempo and time-signature changes across songs (still one song object).
fn song_time_signature_focus() -> Song {
    let mut input =
        String::from("{title: Time Signature Focus}\n{key: C}\n{tempo: 90}\n{time: 3/4}\n");
    for _ in 0..30 {
        input.push_str(
            "{section: Waltz}\n\
             [C]One [G]two [C]three\n\
             [F]One [G]two [C]three\n",
        );
    }
    load_song(&input)
}

// 8. Many short words and spaces to stress spacing normalization.
fn song_spacing_stress() -> Song {
    let mut input = String::from("{title: Spacing Stress}\n{key: C}\n");
    for _ in 0..30 {
        input.push_str(
            "{section: Verse}\n\
             [C]A  a   a    a [G]a  a   a    a\n\
             [Am]B   b  b   b [F]b   b  b   b\n",
        );
    }
    load_song(&input)
}

// 9. Nashville-style chords to stress chord formatting.
fn song_nashville() -> Song {
    let mut input = String::from("{title: Nashville Focus}\n{key: C}\n");
    for _ in 0..30 {
        input.push_str(
            "{section: Verse}\n\
             [1]Line [4]with [5]numbers\n\
             [1m]Another [4]line [5]\n",
        );
    }
    load_song(&input)
}

// 10. Large single section approximating a full page of lyrics + chords.
fn song_full_page_like() -> Song {
    let mut input = String::from("{title: Full Page Like}\n{key: G}\n{section: Story}\n");
    for _ in 0..150 {
        input.push_str("[G]This is a [D]longish [Em]story [C]line of text\n");
    }
    load_song(&input)
}

fn bench_render_song(b: &mut Bencher, song: Song) {
    let rep = ChordRepresentation::Default;
    b.iter(|| {
        let html = (&song).format_html(None, Some(&rep), None, None);
        black_box(html);
    });
}

#[bench]
fn bench_html_render_mixed_standard(b: &mut Bencher) {
    bench_render_song(b, song_mixed_standard());
}

#[bench]
fn bench_html_render_many_sections(b: &mut Bencher) {
    bench_render_song(b, song_many_sections());
}

#[bench]
fn bench_html_render_long_lines_dense_chords(b: &mut Bencher) {
    bench_render_song(b, song_long_lines_dense_chords());
}

#[bench]
fn bench_html_render_chord_only_progression(b: &mut Bencher) {
    bench_render_song(b, song_chord_only_progression());
}

#[bench]
fn bench_html_render_multilingual(b: &mut Bencher) {
    bench_render_song(b, song_multilingual());
}

#[bench]
fn bench_html_render_comment_heavy(b: &mut Bencher) {
    bench_render_song(b, song_comment_heavy());
}

#[bench]
fn bench_html_render_time_signature_focus(b: &mut Bencher) {
    bench_render_song(b, song_time_signature_focus());
}

#[bench]
fn bench_html_render_spacing_stress(b: &mut Bencher) {
    bench_render_song(b, song_spacing_stress());
}

#[bench]
fn bench_html_render_nashville(b: &mut Bencher) {
    bench_render_song(b, song_nashville());
}

#[bench]
fn bench_html_render_full_page_like(b: &mut Bencher) {
    bench_render_song(b, song_full_page_like());
}

