//! Tests for bibliography formatting.
//!
//! Integration tests for bibliography output: ordering, deduplication,
//! and characterization tests for upstream csl_proc behavior.

mod common;

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

// =============================================================================
// Bibliography ordering tests (Issue #4)
// =============================================================================

/// Bibliography order follows citation appearance order, not JSON array order.
#[test]
fn test_bibliography_order_matches_citation_appearance() {
    // Given: 3 refs in JSON order [C, A, B], but cited in order [A, B, C]
    let refs = r#"[
        {"id": "charlie", "type": "article-journal", "author": [{"family": "Charlie", "given": "C."}], "title": "Charlie Title", "issued": {"date-parts": [[2022]]}},
        {"id": "alpha", "type": "article-journal", "author": [{"family": "Alpha", "given": "A."}], "title": "Alpha Title", "issued": {"date-parts": [[2020]]}},
        {"id": "bravo", "type": "article-journal", "author": [{"family": "Bravo", "given": "B."}], "title": "Bravo Title", "issued": {"date-parts": [[2021]]}}
    ]"#;
    let citations = vec![
        Citation {
            id: "alpha".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 10),
        },
        Citation {
            id: "bravo".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (20, 30),
        },
        Citation {
            id: "charlie".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (40, 50),
        },
    ];

    // When: We format the bibliography with a numeric style (no sort in bibliography)
    let result = format_bibliography(&citations, refs, common::NUMERIC_STYLE).unwrap();

    // Then: Order in bibliography should be Alpha < Bravo < Charlie (citation appearance order)
    let alpha_pos = result.find("Alpha").expect("Alpha should appear");
    let bravo_pos = result.find("Bravo").expect("Bravo should appear");
    let charlie_pos = result.find("Charlie").expect("Charlie should appear");
    assert!(
        alpha_pos < bravo_pos && bravo_pos < charlie_pos,
        "Bibliography should follow citation order: Alpha(1) < Bravo(2) < Charlie(3). Got:\n{}",
        result
    );
}

/// When a CSL style has `<sort>` inside `<bibliography>`, csl_proc re-sorts
/// entries regardless of the order we pass refs in. This is correct CSL behavior:
/// the style has the last word on bibliography ordering.
#[test]
fn test_bibliography_sort_override_by_style() {
    // A numeric style WITH <sort><key variable="author"/></sort> in <bibliography>
    let sorted_style = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <id>numeric-sorted</id>
    <title>Numeric Sorted by Author</title>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation collapse="citation-number">
    <sort><key variable="citation-number"/></sort>
    <layout prefix="(" suffix=")" delimiter=",">
      <text variable="citation-number"/>
    </layout>
  </citation>
  <bibliography>
    <sort><key macro="author-sort"/></sort>
    <layout suffix=".">
      <text variable="citation-number" suffix=". "/>
      <names variable="author"><name/></names>
      <text prefix=". " variable="title"/>
    </layout>
  </bibliography>
  <macro name="author-sort">
    <names variable="author"><name name-as-sort-order="all"/></names>
  </macro>
</style>"#;

    // Given: Refs cited in order [Charlie, Alpha] — Charlie is cited first
    let refs = r#"[
        {"id": "charlie", "type": "article-journal", "author": [{"family": "Charlie", "given": "C."}], "title": "Charlie Title", "issued": {"date-parts": [[2022]]}},
        {"id": "alpha", "type": "article-journal", "author": [{"family": "Alpha", "given": "A."}], "title": "Alpha Title", "issued": {"date-parts": [[2020]]}}
    ]"#;
    let citations = vec![
        Citation {
            id: "charlie".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 10),
        },
        Citation {
            id: "alpha".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (20, 30),
        },
    ];

    // When: We format the bibliography with a style that sorts by author
    let result = format_bibliography(&citations, refs, sorted_style).unwrap();

    // Then: Alpha should appear before Charlie (sorted by author name),
    // even though Charlie was cited first
    let alpha_pos = result.find("Alpha").expect("Alpha should appear");
    let charlie_pos = result.find("Charlie").expect("Charlie should appear");
    assert!(
        alpha_pos < charlie_pos,
        "Style <sort> should override citation order: Alpha < Charlie. Got:\n{}",
        result
    );
}

// =============================================================================
// Vancouver characterization: et-al truncation (et-al-min="7" et-al-use-first="6")
// =============================================================================

#[test]
fn test_vancouver_et_al_with_seven_authors() {
    // Given: A reference with exactly 7 authors (triggers et-al-min="7")
    let refs = r#"[{
        "id": "multi2024",
        "type": "article-journal",
        "author": [
            {"family": "Author1", "given": "A"},
            {"family": "Author2", "given": "B"},
            {"family": "Author3", "given": "C"},
            {"family": "Author4", "given": "D"},
            {"family": "Author5", "given": "E"},
            {"family": "Author6", "given": "F"},
            {"family": "Author7", "given": "G"}
        ],
        "title": "Multi-author study",
        "container-title": "J Test",
        "issued": {"date-parts": [[2024]]}
    }]"#;

    // When: We format the bibliography with Vancouver style
    let result = vancouver_bibliography(refs, &["multi2024"]);

    // Then: First 6 authors should appear, 7th should be replaced by "et al."
    assert!(
        result.contains("Author1"),
        "First author should appear, got:\n{}",
        result
    );
    assert!(
        result.contains("Author6"),
        "Sixth author should appear, got:\n{}",
        result
    );
    assert!(
        !result.contains("Author7"),
        "Seventh author should NOT appear (replaced by et al.), got:\n{}",
        result
    );
    assert!(
        result.contains("et al"),
        "Should contain 'et al' for 7+ authors, got:\n{}",
        result
    );
}

#[test]
fn test_vancouver_no_et_al_with_six_authors() {
    // Given: A reference with exactly 6 authors (below et-al-min="7")
    let refs = r#"[{
        "id": "six2024",
        "type": "article-journal",
        "author": [
            {"family": "Author1", "given": "A"},
            {"family": "Author2", "given": "B"},
            {"family": "Author3", "given": "C"},
            {"family": "Author4", "given": "D"},
            {"family": "Author5", "given": "E"},
            {"family": "Author6", "given": "F"}
        ],
        "title": "Six-author study",
        "container-title": "J Test",
        "issued": {"date-parts": [[2024]]}
    }]"#;

    // When: We format the bibliography with Vancouver style
    let result = vancouver_bibliography(refs, &["six2024"]);

    // Then: All 6 authors should appear, no "et al."
    assert!(
        result.contains("Author6"),
        "All 6 authors should appear, got:\n{}",
        result
    );
    assert!(
        !result.contains("et al"),
        "Should NOT contain 'et al' for 6 authors, got:\n{}",
        result
    );
}
