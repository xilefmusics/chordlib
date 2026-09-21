#![cfg(feature = "bin")]

use std::fs;
use std::path::{Path, PathBuf};
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
            std::env::temp_dir().join(format!("chordlib-markdown-{}-{unique}", std::process::id()));
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

#[test]
fn cli_routes_markdown_extensions_case_insensitively() {
    let directory = TestDir::new();
    let input = directory.join("input.SoNg.MD");
    let output = directory.join("output.MARKDOWN");
    fs::write(
        &input,
        "---\ntitles: [CLI]\nkey: C\n---\n# Verse (2x)\nC\nText\n",
    )
    .expect("write input");

    let result = Command::new(env!("CARGO_BIN_EXE_chordlib"))
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .expect("run chordlib");
    assert!(
        result.status.success(),
        "markdown CLI failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let exported = fs::read_to_string(output).expect("read Markdown output");
    assert!(exported.starts_with("---\n"));
    assert!(exported.contains("# Verse (2x)"));
    assert!(exported.contains("Text"));
}

#[test]
fn cli_can_convert_markdown_to_chordpro() {
    let directory = TestDir::new();
    let input = directory.join("input.md");
    let output = directory.join("output.cp");
    fs::write(
        &input,
        "---\ntitles: [CLI]\nkey: C\n---\n# Verse\nC       G\nAmazing grace\n",
    )
    .expect("write input");

    let result = Command::new(env!("CARGO_BIN_EXE_chordlib"))
        .arg(&input)
        .arg("--output")
        .arg(Path::new(&output))
        .output()
        .expect("run chordlib");
    assert!(
        result.status.success(),
        "markdown import failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(output).expect("read ChordPro output");
    assert!(exported.contains("[C]Amazing [G]grace"), "{exported}");
}
