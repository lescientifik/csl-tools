//! CLI integration tests.
//!
//! Tests the command-line interface by running the binary as a subprocess.

mod common;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Path to the compiled binary
fn binary_path() -> PathBuf {
    // The binary is built in target/debug or target/release
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("csl-tools");
    path
}

/// Helper to create a temporary file with content
fn create_temp_file(content: &str, extension: &str) -> NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(extension)
        .tempfile()
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

/// Minimal CSL style for testing
const TEST_STYLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <title>Test Style</title>
    <id>test-style</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <layout prefix="(" suffix=")" delimiter="; ">
      <names variable="author">
        <name form="short"/>
      </names>
      <date prefix=", " variable="issued"><date-part name="year"/></date>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <names variable="author"><name name-as-sort-order="all"/></names>
      <text prefix=". " variable="title" font-style="italic"/>
      <date prefix=" (" suffix=")." variable="issued"><date-part name="year"/></date>
    </layout>
  </bibliography>
</style>"#;

const TEST_REFS: &str = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]"#;

// ============================================
// Tests for CLI argument parsing
// ============================================

#[test]
fn test_cli_help() {
    // Given: The CLI binary
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    // Then: Help is displayed with expected content
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("csl-tools") || stdout.contains("Format citations"),
        "Help should mention the tool name or purpose: {}",
        stdout
    );
    assert!(output.status.success(), "Help should exit with success");
}

#[test]
fn test_cli_process_subcommand_help() {
    // Given: The process subcommand
    let output = Command::new(binary_path())
        .args(["process", "--help"])
        .output()
        .expect("Failed to execute command");

    // Then: Process help is displayed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bib") || stdout.contains("--bib"),
        "Process help should mention --bib option: {}",
        stdout
    );
    assert!(
        stdout.contains("csl") || stdout.contains("--csl"),
        "Process help should mention --csl option: {}",
        stdout
    );
    assert!(
        output.status.success(),
        "Process help should exit with success"
    );
}

#[test]
fn test_cli_process_missing_args() {
    // Given: The process subcommand without required arguments
    let output = Command::new(binary_path())
        .args(["process"])
        .output()
        .expect("Failed to execute command");

    // Then: Error is displayed about missing arguments
    assert!(!output.status.success(), "Process without args should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("error") || stderr.contains("Usage"),
        "Should indicate missing required arguments: {}",
        stderr
    );
}

// ============================================
// Tests for process command
// ============================================

#[test]
fn test_cli_process_basic() {
    // Given: Markdown file with a citation, refs file, and style file
    let markdown = "Les résultats montrent [@item-1] que la méthode fonctionne.";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We run the process command
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output contains the formatted citation
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("(Doe, 2021)"),
        "Output should contain formatted citation: {}",
        stdout
    );
    // And the bibliography
    assert!(
        stdout.contains("References") || stdout.contains("csl-bib-body"),
        "Output should contain bibliography section: {}",
        stdout
    );
}

#[test]
fn test_cli_process_no_bib() {
    // Given: Markdown file with a citation
    let markdown = "Les résultats montrent [@item-1] que la méthode fonctionne.";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We run the process command with --no-bib
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "--no-bib",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output contains the formatted citation but no bibliography
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("(Doe, 2021)"),
        "Output should contain formatted citation: {}",
        stdout
    );
    assert!(
        !stdout.contains("csl-bib-body"),
        "Output should NOT contain bibliography when --no-bib is used: {}",
        stdout
    );
}

#[test]
fn test_cli_process_custom_bib_header() {
    // Given: Markdown file with a citation
    let markdown = "Les résultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We run the process command with custom bib header
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "--bib-header",
            "## Bibliography",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output uses the custom header
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("## Bibliography"),
        "Output should contain custom bibliography header: {}",
        stdout
    );
}

#[test]
fn test_cli_process_output_file() {
    // Given: Markdown file with a citation and an output file path
    let markdown = "Les résultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");
    let output_file = tempfile::Builder::new().suffix(".md").tempfile().unwrap();

    // When: We run the process command with -o output file
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            output_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output is written to the file (stdout should be empty or minimal)
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And the file contains the formatted output
    let file_content = fs::read_to_string(output_file.path()).unwrap();
    assert!(
        file_content.contains("(Doe, 2021)"),
        "Output file should contain formatted citation: {}",
        file_content
    );
}

#[test]
fn test_cli_process_missing_input_file() {
    // Given: Non-existent input file
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We try to process a non-existent file
    let output = Command::new(binary_path())
        .args([
            "process",
            "/nonexistent/path/article.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: An error is displayed
    assert!(
        !output.status.success(),
        "Process should fail for non-existent input file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error")
            || stderr.contains("Error")
            || stderr.contains("not found")
            || stderr.contains("No such file"),
        "Should indicate file not found error: {}",
        stderr
    );
}

#[test]
fn test_cli_process_missing_bib_file() {
    // Given: Valid input file but non-existent refs file
    let markdown = "Les résultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We try to process with non-existent bib file
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            "/nonexistent/refs.json",
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: An error is displayed
    assert!(
        !output.status.success(),
        "Process should fail for non-existent bib file"
    );
}

#[test]
fn test_cli_process_builtin_style() {
    // Given: Markdown file with a citation, using builtin style name
    let markdown = "Les résultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    // When: We run with builtin style name "minimal"
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "minimal",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output is formatted (builtin style should work)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Process with builtin style should succeed. stderr: {}",
        stderr
    );
    assert!(
        stdout.contains("Doe"),
        "Output should contain author name: {}",
        stdout
    );
}

#[test]
fn test_cli_process_jsonl_refs() {
    // Given: References in JSONL format (one JSON object per line)
    let jsonl_refs = r#"{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}
{"id": "item-2", "type": "book", "author": [{"family": "Smith", "given": "Jane"}], "title": "Another Book", "issued": {"date-parts": [[2022]]}}"#;
    let markdown = "First [@item-1] and second [@item-2].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(jsonl_refs, ".jsonl");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: We run the process command
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: Both citations are formatted
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed with JSONL. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Doe") && stdout.contains("Smith"),
        "Output should contain both authors: {}",
        stdout
    );
}

// ============================================
// Tests for citation grouping (Phase 6)
// ============================================

const GROUPED_TEST_REFS: &str = r#"[
    {"id": "ref-a", "type": "article-journal", "author": [{"family": "AuthorA", "given": "A."}], "title": "Title A", "issued": {"date-parts": [[2020]]}},
    {"id": "ref-b", "type": "article-journal", "author": [{"family": "AuthorB", "given": "B."}], "title": "Title B", "issued": {"date-parts": [[2021]]}},
    {"id": "ref-c", "type": "article-journal", "author": [{"family": "AuthorC", "given": "C."}], "title": "Title C", "issued": {"date-parts": [[2022]]}}
]"#;

#[test]
fn test_cli_process_grouped_citations() {
    // Given: Markdown with adjacent citations that should be grouped
    let markdown = "Studies [@ref-a] [@ref-b] [@ref-c] show that...";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(GROUPED_TEST_REFS, ".json");
    let style_file = create_temp_file(common::NUMERIC_STYLE, ".csl");

    // When: We run the process command
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The citations should be grouped as (1-3) or (1,2,3), NOT (1) (2) (3)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The key assertion: grouped citations should NOT produce "(1) (2) (3)"
    assert!(
        !stdout.contains("(1) (2) (3)") && !stdout.contains("(1)(2)(3)"),
        "Adjacent citations should be grouped, not separate. Got: {}",
        stdout
    );

    // Should contain all three numbers in a single citation (grouped)
    // Either "(1-3)" (collapsed) or "(1,2,3)" (not collapsed but grouped)
    // Note: en-dash (–) may be used instead of hyphen (-)
    let has_grouped = stdout.contains("(1-3)")
        || stdout.contains("(1–3)")
        || stdout.contains("(1,2,3)")
        || stdout.contains("(1, 2, 3)");
    assert!(
        has_grouped,
        "Citations should be grouped as (1-3) or (1,2,3), got: {}",
        stdout
    );
}

#[test]
fn test_cli_process_mixed_grouped_and_separate() {
    // Given: Markdown with some adjacent citations and some separate
    let markdown = "First group [@ref-a] [@ref-b] and then separate [@ref-c].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(GROUPED_TEST_REFS, ".json");
    let style_file = create_temp_file(common::NUMERIC_STYLE, ".csl");

    // When: We run the process command
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: First two should be grouped, third separate
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // First group: (1-2) or (1,2)
    let has_first_group =
        stdout.contains("(1-2)") || stdout.contains("(1–2)") || stdout.contains("(1,2)");
    // Third citation separate: (3)
    let has_separate_third = stdout.contains("(3)");

    assert!(
        has_first_group,
        "First two citations should be grouped, got: {}",
        stdout
    );
    assert!(
        has_separate_third,
        "Third citation should be separate as (3), got: {}",
        stdout
    );
}

// ============================================
// Tests for exit codes (semantic: 10-15)
// ============================================

#[test]
fn test_exit_code_10_input_file_not_found() {
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            "/nonexistent/path/article.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output.status.code(),
        Some(10),
        "Missing input file should exit with code 10, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_11_bib_file_not_found() {
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            "/nonexistent/refs.json",
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output.status.code(),
        Some(11),
        "Missing bib file should exit with code 11, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_12_style_not_found() {
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "nonexistent-style-name",
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output.status.code(),
        Some(12),
        "Unknown style should exit with code 12, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_13_reference_not_found() {
    let markdown = "See [@unknown-key].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output.status.code(),
        Some(13),
        "Unknown citation key should exit with code 13, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_15_output_dir_not_writable() {
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            "/nonexistent/dir/output.md",
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output.status.code(),
        Some(15),
        "Unwritable output path should exit with code 15, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================
// Tests for stdin support
// ============================================

#[test]
fn test_stdin_support() {
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");
    let markdown_input = "Les resultats montrent [@item-1] que la methode fonctionne.";

    let mut child = Command::new(binary_path())
        .args([
            "process",
            "-",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(markdown_input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process from stdin should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Doe"),
        "Output should contain formatted citation from stdin input: {}",
        stdout
    );
}

// ============================================
// Tests for styles subcommand
// ============================================

#[test]
fn test_styles_subcommand() {
    let output = Command::new(binary_path())
        .arg("styles")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "styles subcommand should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("minimal"),
        "styles output should list 'minimal' builtin style, got: {}",
        stdout
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "styles subcommand should exit with code 0"
    );
}

// ============================================
// Tests for Vancouver builtin style
// ============================================

#[test]
fn test_styles_subcommand_lists_vancouver() {
    // Given: The styles subcommand
    let output = Command::new(binary_path())
        .arg("styles")
        .output()
        .expect("Failed to execute command");

    // Then: Vancouver is listed among builtin styles
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "styles subcommand should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("vancouver"),
        "styles output should list 'vancouver' builtin style, got: {}",
        stdout
    );
}

#[test]
fn test_cli_process_builtin_vancouver() {
    // Given: Markdown with a citation, using builtin vancouver style
    let markdown = "Les résultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    // When: We run with builtin style name "vancouver"
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "vancouver",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: The output contains a numeric citation (1)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process with vancouver style should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("(1)"),
        "Vancouver style should produce numeric citation (1), got: {}",
        stdout
    );
}

// ============================================
// Tests for error hints
// ============================================

#[test]
fn test_error_hint_input_file() {
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            "/nonexistent/article.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint:"),
        "stderr should contain a hint, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_style_lists_builtin_names() {
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "totally-fake-style",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("available builtin styles:"),
        "stderr should list available builtin styles, got: {}",
        stderr
    );
    assert!(
        stderr.contains("minimal"),
        "stderr should mention 'minimal' as available style, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_reference_not_found() {
    let markdown = "See [@nonexistent-key].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint: check that this citation key exists"),
        "stderr should contain reference-not-found hint, got: {}",
        stderr
    );
}

// ============================================
// Tests for confirmation message on stderr
// ============================================

#[test]
fn test_success_confirmation_message_on_stderr() {
    let markdown = "Les resultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");
    let output_file = tempfile::Builder::new().suffix(".md").tempfile().unwrap();

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            output_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Process should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("processed") && stderr.contains("wrote"),
        "stderr should contain confirmation with 'processed' and 'wrote', got: {}",
        stderr
    );
}

#[test]
fn test_no_confirmation_message_on_stdout_output() {
    let markdown = "Les resultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Process should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("processed"),
        "stderr should NOT contain confirmation when output goes to stdout, got: {}",
        stderr
    );
}

