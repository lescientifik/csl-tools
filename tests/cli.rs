//! CLI integration tests.
//!
//! Tests the command-line interface by running the binary as a subprocess.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
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

// Numeric style with collapse for testing grouped citations
const NUMERIC_COLLAPSE_STYLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <title>Numeric Collapse</title>
    <id>numeric-collapse</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation collapse="citation-number">
    <sort>
      <key variable="citation-number"/>
    </sort>
    <layout prefix="(" suffix=")" delimiter=",">
      <text variable="citation-number"/>
    </layout>
  </citation>
  <bibliography>
    <layout suffix=".">
      <text variable="citation-number" suffix=". "/>
      <names variable="author"><name/></names>
      <text prefix=". " variable="title"/>
    </layout>
  </bibliography>
</style>"#;

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
    let style_file = create_temp_file(NUMERIC_COLLAPSE_STYLE, ".csl");

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
    let style_file = create_temp_file(NUMERIC_COLLAPSE_STYLE, ".csl");

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
// Tests for skill-install command
// ============================================

#[test]
fn test_cli_skill_install_help() {
    // Given: The skill-install subcommand
    let output = Command::new(binary_path())
        .args(["skill-install", "--help"])
        .output()
        .expect("Failed to execute command");

    // Then: Help is displayed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--dir") || stdout.contains("-d"),
        "skill-install help should mention --dir option: {}",
        stdout
    );
    assert!(
        output.status.success(),
        "skill-install help should exit with success"
    );
}

#[test]
fn test_cli_skill_install_creates_files() {
    // Given: A temporary directory
    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("my-skills");

    // When: We run skill-install with custom directory
    let output = Command::new(binary_path())
        .args(["skill-install", "--dir", skills_dir.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    // Then: The command succeeds
    assert!(
        output.status.success(),
        "skill-install should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And the skill file is created
    let skill_file = skills_dir.join("csl-format").join("SKILL.md");
    assert!(skill_file.exists(), "SKILL.md should be created");

    // And it contains the expected content
    let content = fs::read_to_string(&skill_file).unwrap();
    assert!(
        content.contains("name: csl-format"),
        "SKILL.md should contain the skill name"
    );
    assert!(
        content.contains("csl-tools"),
        "SKILL.md should reference csl-tools"
    );
}

#[test]
fn test_cli_skill_install_default_directory() {
    // Given: A temporary directory to run in
    let temp_dir = tempfile::tempdir().unwrap();

    // When: We run skill-install without --dir (uses default .claude/skills)
    let output = Command::new(binary_path())
        .args(["skill-install"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Then: The command succeeds
    assert!(
        output.status.success(),
        "skill-install should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // And the skill file is created in default location
    let skill_file = temp_dir
        .path()
        .join(".claude/skills/csl-format/SKILL.md");
    assert!(
        skill_file.exists(),
        "SKILL.md should be created in default location"
    );
}
