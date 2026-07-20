#![cfg(feature = "bin")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "chordlib-propresenter-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chordlib"));
    for argument in args {
        command.arg(argument);
    }
    command.output().expect("run chordlib CLI")
}

#[test]
fn converts_mixed_case_chordpro_to_pro_and_selects_language_and_key() {
    let directory = TestDir::new("round-trip");
    let input = directory.join("input.WP");
    let presentation = directory.join("output.PrO");
    let result = directory.join("result.cP");
    fs::write(
        &input,
        "{title: Languages}\n{language: de en}\n{key: C}\n{section: Verse}\n[C]Hallo\n&[C]Hello\n",
    )
    .expect("write input");

    let output = Command::new(env!("CARGO_BIN_EXE_chordlib"))
        .arg(&input)
        .arg("--output")
        .arg(&presentation)
        .arg("--language")
        .arg("1")
        .arg("--key")
        .arg("5")
        .arg("--nashville")
        .output()
        .expect("export ProPresenter");
    assert!(
        output.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::metadata(&presentation).expect("presentation").len() > 0);

    let output = run(&[&presentation, Path::new("--output"), &result]);
    assert!(
        output.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let converted = fs::read_to_string(result).expect("read converted ChordPro");
    assert!(converted.contains("Hello"));
    assert!(!converted.contains("Hallo"));
    assert!(converted.contains("[D]"), "converted output: {converted}");
}

#[test]
fn routes_chopro_as_text_and_reports_malformed_protobuf() {
    let directory = TestDir::new("routing");
    let chordpro = directory.join("song.ChOpRo");
    let presentation = directory.join("song.PRO");
    fs::write(
        &chordpro,
        "{title: Routed}\n{key: C}\n{section: Verse}\n[C]Text\n",
    )
    .expect("write ChordPro");
    let output = run(&[&chordpro, Path::new("--output"), &presentation]);
    assert!(
        output.status.success(),
        "ChordPro route failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let malformed = directory.join("malformed.pro");
    let target = directory.join("unused.cp");
    fs::write(&malformed, b"not a protobuf").expect("write malformed input");
    let output = run(&[&malformed, Path::new("--output"), &target]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid ProPresenter protobuf"));
}
