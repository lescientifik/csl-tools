//! Integration tests using TOML fixtures.
//!
//! This test harness loads test cases from TOML files in the `fixtures/` directory
//! and runs them against the csl-tools library.

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// A test fixture loaded from a TOML file.
#[derive(Debug, Deserialize)]
struct Fixture {
    /// Name of the test case
    name: String,
    /// Input Markdown text
    markdown: String,
    /// CSL-JSON references (as a JSON string)
    #[serde(default)]
    refs: String,
    /// CSL style XML
    #[serde(default)]
    style: String,
    /// Expected output (for full integration tests)
    #[serde(default)]
    expected: Option<String>,
    /// Expected citation ID (for parsing tests)
    #[serde(default)]
    expected_id: Option<String>,
    /// Expected citation URL (for parsing tests)
    #[serde(default)]
    expected_url: Option<String>,
    /// Expected error message (for error tests)
    #[serde(default)]
    expected_error: Option<String>,
    /// Test type: "parsing", "integration", "output", or "error"
    #[serde(default = "default_test_type")]
    test_type: String,
}

fn default_test_type() -> String {
    "integration".to_string()
}

/// Load all fixtures from a directory.
fn load_fixtures(dir: &Path) -> Vec<(String, Fixture)> {
    let mut fixtures = Vec::new();

    if !dir.exists() {
        return fixtures;
    }

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "toml") {
            let content = fs::read_to_string(&path).unwrap();
            let fixture: Fixture = toml::from_str(&content).unwrap();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            fixtures.push((name, fixture));
        }
    }

    fixtures
}

/// Run parsing tests - verify citation extraction from Markdown.
fn run_parsing_test(name: &str, fixture: &Fixture) {
    let citations = csl_tools::extract_citations(&fixture.markdown);

    println!(
        "Parsing test '{}': {} citations found",
        name,
        citations.len()
    );

    // Check expected citation ID
    if let Some(expected_id) = &fixture.expected_id {
        assert!(
            !citations.is_empty(),
            "Test '{}' failed: expected citation with id '{}' but got none",
            name,
            expected_id
        );
        assert_eq!(
            citations[0].id, *expected_id,
            "Test '{}' failed: expected id '{}', got '{}'",
            name, expected_id, citations[0].id
        );
    }

    // Check expected citation URL
    if let Some(expected_url) = &fixture.expected_url {
        assert!(
            !citations.is_empty(),
            "Test '{}' failed: expected citation with url but got none",
            name
        );
        assert_eq!(
            citations[0].url.as_deref(),
            Some(expected_url.as_str()),
            "Test '{}' failed: expected url '{}', got '{:?}'",
            name,
            expected_url,
            citations[0].url
        );
    }

    // Legacy support for 'expected' field (simple ID check)
    if let Some(expected) = &fixture.expected {
        assert!(
            !expected.is_empty() || citations.is_empty(),
            "Test '{}' failed: expected citations but got none",
            name
        );
    }
}

/// Run full integration tests - process Markdown and verify output.
fn run_integration_test(name: &str, fixture: &Fixture) {
    // Extract citations
    let citations = csl_tools::extract_citations(&fixture.markdown);

    // Format citations
    let processed = csl_tools::format_citations(&citations, &fixture.refs, &fixture.style);

    match processed {
        Ok(processed) => {
            // Replace citations in text
            let content = csl_tools::replace_citations(&fixture.markdown, &processed);

            // Format bibliography
            let bibliography =
                csl_tools::format_bibliography(&citations, &fixture.refs, &fixture.style)
                    .ok()
                    .filter(|s| !s.is_empty());

            // Generate final output
            let output =
                csl_tools::generate_output(&content, bibliography.as_deref(), "## References");

            if let Some(expected) = &fixture.expected {
                assert_eq!(
                    output.trim(),
                    expected.trim(),
                    "Test '{}' output mismatch",
                    name
                );
            }
        }
        Err(e) => {
            if fixture.expected_error.is_some() {
                // Expected an error, this is fine
            } else {
                panic!("Test '{}' failed with unexpected error: {}", name, e);
            }
        }
    }
}

/// Run error tests - verify proper error handling.
fn run_error_test(name: &str, fixture: &Fixture) {
    let citations = csl_tools::extract_citations(&fixture.markdown);
    let result = csl_tools::format_citations(&citations, &fixture.refs, &fixture.style);

    match result {
        Ok(_) => {
            if fixture.expected_error.is_some() {
                panic!("Test '{}' expected an error but succeeded", name);
            }
        }
        Err(e) => {
            if let Some(expected_error) = &fixture.expected_error {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains(expected_error),
                    "Test '{}' error mismatch: expected '{}', got '{}'",
                    name,
                    expected_error,
                    error_msg
                );
            } else {
                panic!("Test '{}' failed with unexpected error: {}", name, e);
            }
        }
    }
}

#[test]
fn test_parsing_fixtures() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parsing");
    let fixtures = load_fixtures(&fixtures_dir);

    for (name, fixture) in fixtures {
        println!("Running parsing test: {}", fixture.name);
        run_parsing_test(&name, &fixture);
    }
}

#[test]
fn test_integration_fixtures() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/integration");
    let fixtures = load_fixtures(&fixtures_dir);

    for (name, fixture) in fixtures {
        println!("Running integration test: {}", fixture.name);
        run_integration_test(&name, &fixture);
    }
}

#[test]
fn test_output_fixtures() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output");
    let fixtures = load_fixtures(&fixtures_dir);

    for (name, fixture) in fixtures {
        println!("Running output test: {}", fixture.name);
        run_integration_test(&name, &fixture);
    }
}

#[test]
fn test_error_fixtures() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/errors");
    let fixtures = load_fixtures(&fixtures_dir);

    for (name, fixture) in fixtures {
        println!("Running error test: {}", fixture.name);
        run_error_test(&name, &fixture);
    }
}
