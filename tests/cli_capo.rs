#![cfg(feature = "bin")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("chordlib-capo-{}-{unique}", std::process::id()));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(input: &PathBuf, output: &PathBuf, extra_args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chordlib"));
    command.arg(input).arg("--output").arg(output);
    command.args(extra_args);
    command.output().expect("run chordlib")
}

#[test]
fn capo_preserves_chord_shapes_and_updates_html_and_chordpro_keys() {
    let directory = TestDir::new();
    let input = directory.join("input.cp");
    let html_output = directory.join("output.html");
    let chordpro_output = directory.join("output.cp");
    fs::write(
        &input,
        "{title: Capo}\n{key: G}\n{section: Verse}\n[G]Line\n",
    )
    .expect("write input");

    let result = run(&input, &html_output, &["--capo", "4"]);
    assert!(
        result.status.success(),
        "HTML export failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(html_output).expect("read HTML output");
    assert!(html.contains("Key B"), "{html}");
    assert!(!html.contains("Capo 4"), "{html}");
    assert!(html.contains(r#"<span class="chord">G</span>"#), "{html}");

    let result = run(&input, &chordpro_output, &["--capo", "4"]);
    assert!(
        result.status.success(),
        "ChordPro export failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let chordpro = fs::read_to_string(chordpro_output).expect("read ChordPro output");
    assert!(chordpro.contains("{key:B}"), "{chordpro}");
    assert!(chordpro.contains("[G]Line"), "{chordpro}");
}

#[test]
fn capo_is_applied_after_the_cli_key_override() {
    let directory = TestDir::new();
    let input = directory.join("input.cp");
    let output = directory.join("output.cp");
    fs::write(
        &input,
        "{title: Capo}\n{key: G}\n{section: Verse}\n[G]Line\n",
    )
    .expect("write input");

    let result = run(&input, &output, &["--key", "2", "--capo", "4"]);
    assert!(
        result.status.success(),
        "ChordPro export failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let chordpro = fs::read_to_string(output).expect("read ChordPro output");
    assert!(chordpro.contains("{key:Eb}"), "{chordpro}");
    assert!(chordpro.contains("[B]Line"), "{chordpro}");
}

#[test]
fn capo_updates_metadata_and_preserves_stdout_markdown_and_json_shapes() {
    let directory = TestDir::new();
    let input = directory.join("input.cp");
    let markdown_output = directory.join("output.md");
    let json_output = directory.join("output.json");
    fs::write(
        &input,
        "{title: Capo}\n{key: G}\n{section: Verse}\n[G]Line\n",
    )
    .expect("write input");

    let rendered = Command::new(env!("CARGO_BIN_EXE_chordlib"))
        .arg(&input)
        .arg("--render")
        .arg("--capo")
        .arg("4")
        .output()
        .expect("run chordlib");
    assert!(
        rendered.status.success(),
        "stdout render failed: {}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    assert!(
        String::from_utf8_lossy(&rendered.stdout).contains("\x1b[32;1mG\x1b[0m"),
        "{}",
        String::from_utf8_lossy(&rendered.stdout)
    );

    let result = run(&input, &markdown_output, &["--capo", "4"]);
    assert!(
        result.status.success(),
        "Markdown export failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let markdown = fs::read_to_string(markdown_output).expect("read Markdown output");
    assert!(markdown.contains("key: B"), "{markdown}");
    assert!(markdown.contains("G\nLine"), "{markdown}");

    let result = run(&input, &json_output, &["--capo", "4"]);
    assert!(
        result.status.success(),
        "JSON export failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(json_output).expect("read JSON output"))
            .expect("parse JSON output");
    assert_eq!(json["key"]["level"].as_u64(), Some(2));
    assert_eq!(
        json["sections"][0]["lines"][0]["parts"][0]["chord"]["main"]["level"].as_u64(),
        Some(8)
    );
}
