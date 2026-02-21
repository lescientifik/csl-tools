//! Characterization tests for bibliography formatting.
//!
//! These tests verify that csl_proc correctly formats bibliographic entries
//! with specific styles. They serve as regression guards: if they fail,
//! the issue is in csl_proc (upstream), not in csl-tools.

use csl_tools::{builtin_style, format_bibliography, Citation};

/// Helper: format a bibliography using Vancouver style and return the HTML output.
fn vancouver_bibliography(refs_json: &str, citation_ids: &[&str]) -> String {
    let style = builtin_style("vancouver").expect("vancouver style should be available");
    let citations: Vec<Citation> = citation_ids
        .iter()
        .enumerate()
        .map(|(i, id)| Citation {
            id: id.to_string(),
            locator: None,
            label: None,
            url: None,
            span: (i * 20, i * 20 + 10),
        })
        .collect();
    format_bibliography(&citations, refs_json, style).unwrap()
}

#[test]
fn test_vancouver_author_formatting() {
    // Given: A reference with compound given name "Anne-Ségolène"
    let refs = r#"[{
        "id": "cottereau2023",
        "type": "article-journal",
        "author": [{"family": "Cottereau", "given": "Anne-Ségolène"}],
        "title": "Test Article",
        "container-title": "J Test",
        "issued": {"date-parts": [[2023]]}
    }]"#;

    // When: We format the bibliography with Vancouver style
    let result = vancouver_bibliography(refs, &["cottereau2023"]);

    // Then: Author should be initialized (not full given name)
    // csl_proc renders "Anne-Ségolène" as "A-S" (preserving hyphen in compound initials)
    assert!(
        result.contains("Cottereau A-S"),
        "Vancouver should format 'Anne-Ségolène Cottereau' as 'Cottereau A-S', got:\n{}",
        result
    );
    assert!(
        !result.contains("Anne-Ségolène"),
        "Vancouver should NOT show full given name, got:\n{}",
        result
    );
}

#[test]
fn test_vancouver_multiple_authors() {
    // Given: A reference with multiple Chinese-style authors
    let refs = r#"[{
        "id": "hu2020",
        "type": "article-journal",
        "author": [
            {"family": "Hu", "given": "Ben"},
            {"family": "Guo", "given": "Hua"},
            {"family": "Zhou", "given": "Peng"}
        ],
        "title": "Discovery of a novel coronavirus",
        "container-title": "Nature",
        "issued": {"date-parts": [[2020]]}
    }]"#;

    // When: We format the bibliography with Vancouver style
    let result = vancouver_bibliography(refs, &["hu2020"]);

    // Then: All authors should have initialized given names
    assert!(
        result.contains("Hu B"),
        "Expected 'Hu B' in bibliography, got:\n{}",
        result
    );
    assert!(
        result.contains("Guo H"),
        "Expected 'Guo H' in bibliography, got:\n{}",
        result
    );
    assert!(
        result.contains("Zhou P"),
        "Expected 'Zhou P' in bibliography, got:\n{}",
        result
    );
}

#[test]
fn test_vancouver_numeric_citations() {
    // Given: Two references cited in order
    let refs = r#"[
        {"id": "ref1", "type": "article-journal", "author": [{"family": "Smith", "given": "John"}], "title": "First Article", "container-title": "J First", "issued": {"date-parts": [[2020]]}},
        {"id": "ref2", "type": "article-journal", "author": [{"family": "Jones", "given": "Jane"}], "title": "Second Article", "container-title": "J Second", "issued": {"date-parts": [[2021]]}}
    ]"#;

    // When: We format the bibliography with Vancouver style
    let result = vancouver_bibliography(refs, &["ref1", "ref2"]);

    // Then: Entries should be numbered 1. and 2.
    assert!(
        result.contains("1. "),
        "First entry should be numbered '1. ', got:\n{}",
        result
    );
    assert!(
        result.contains("2. "),
        "Second entry should be numbered '2. ', got:\n{}",
        result
    );
}
