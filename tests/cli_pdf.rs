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
            std::env::temp_dir().join(format!("chordlib-pdf-{}-{unique}", std::process::id()));
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

fn generated_pdf() -> Vec<u8> {
    let content = concat!(
        "BT /F1 18 Tf 72 750 Td (CLI Song) Tj ET\n",
        "BT /F1 10 Tf 72 732 Td (CLI Artist) Tj ET\n",
        "BT /F1 10 Tf 72 716 Td (Key - G | Tempo - 100 | Time - 4/4) Tj ET\n",
        "BT /F1 12 Tf 72 690 Td (VERSE) Tj ET\n",
        "BT /F1 14 Tf 72 676 Td (G) Tj ET\n",
        "BT /F1 14 Tf 72 664 Td (Hello PDF) Tj ET\n",
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [4 0 R] /Count 1 >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents 5 0 R >>".to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ),
    ];

    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn run(input: &Path, output: &Path) {
    let result = Command::new(env!("CARGO_BIN_EXE_chordlib"))
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .expect("run chordlib CLI");
    assert!(
        result.status.success(),
        "PDF CLI failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn routes_uppercase_pdf_extension_to_chordpro() {
    let directory = TestDir::new();
    let input = directory.join("song.PDF");
    let output = directory.join("song.CP");
    fs::write(&input, generated_pdf()).expect("write generated PDF");

    run(&input, &output);

    let chordpro = fs::read_to_string(output).expect("read ChordPro output");
    assert!(chordpro.contains("{title:CLI Song}"), "{chordpro}");
    assert!(chordpro.contains("[G]Hello PDF"), "{chordpro}");
}

#[test]
fn routes_mixed_case_pdf_extension_to_markdown() {
    let directory = TestDir::new();
    let input = directory.join("song.PdF");
    let output = directory.join("song.Md");
    fs::write(&input, generated_pdf()).expect("write generated PDF");

    run(&input, &output);

    let markdown = fs::read_to_string(output).expect("read Markdown output");
    assert!(markdown.contains("titles:\n- CLI Song"), "{markdown}");
    assert!(markdown.contains("# VERSE"), "{markdown}");
    assert!(markdown.contains("Hello PDF"), "{markdown}");
}
