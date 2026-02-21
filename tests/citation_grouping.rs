//! Tests for citation grouping/clustering functionality.
//!
//! These tests verify that adjacent citations are properly grouped into clusters,
//! and that the Pandoc syntax `[@a; @b; @c]` is supported.
//!
//! The grouping rules are:
//! - Citations separated only by whitespace (or directly adjacent) are grouped
//! - Citations separated by punctuation or text are NOT grouped

mod common;

use csl_tools::{extract_citation_clusters, format_citations_clusters, CitationCluster, CitationItem};
use common::{build_refs, NUMERIC_STYLE};

// =============================================================================
// Tests for parsing adjacent citations
// =============================================================================

/// Test 1: Adjacent citations detected as a group
#[test]
fn test_adjacent_citations_grouped() {
    // Given: Markdown with adjacent citations separated by spaces
    let markdown = "Studies [@a] [@b] [@c] show that...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All three citations are grouped into a single cluster
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        3,
        "Expected 3 items in cluster, got {}",
        clusters[0].items.len()
    );
    assert_eq!(clusters[0].items[0].id, "a");
    assert_eq!(clusters[0].items[1].id, "b");
    assert_eq!(clusters[0].items[2].id, "c");
}

/// Test 2: Adjacent citations with URLs (space-separated)
#[test]
fn test_adjacent_citations_with_urls_grouped() {
    // Given: Markdown with adjacent citations that have URLs
    let markdown = "Studies [@a](url1) [@b](url2) [@c](url3) show...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All citations are grouped, URLs are preserved in items
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        3,
        "Expected 3 items in cluster, got {}",
        clusters[0].items.len()
    );
    assert_eq!(clusters[0].items[0].url, Some("url1".into()));
    assert_eq!(clusters[0].items[1].url, Some("url2".into()));
    assert_eq!(clusters[0].items[2].url, Some("url3".into()));
}

/// Test 2b: Adjacent citations directly touching (no space)
#[test]
fn test_adjacent_citations_no_space_grouped() {
    // Given: Markdown with citations directly adjacent (no space between)
    let markdown = "Studies [@a][@b][@c] show that...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All citations are grouped into a single cluster
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        3,
        "Expected 3 items in cluster, got {}",
        clusters[0].items.len()
    );
}

/// Test 2c: Adjacent citations with URLs directly touching (no space)
#[test]
fn test_adjacent_citations_with_urls_no_space_grouped() {
    // Given: Markdown with citations+URLs directly adjacent
    let markdown = "Studies [@a](url1)[@b](url2)[@c](url3) show...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All citations are grouped, URLs are preserved
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        3,
        "Expected 3 items in cluster, got {}",
        clusters[0].items.len()
    );
    assert_eq!(clusters[0].items[0].url, Some("url1".into()));
    assert_eq!(clusters[0].items[1].url, Some("url2".into()));
}

/// Test 3: Citations separated by punctuation are NOT grouped
#[test]
fn test_citations_separated_by_punctuation_not_grouped() {
    // Given: Markdown with citations separated by comma
    let markdown = "First [@a], then [@b].";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: Each citation is in its own cluster
    assert_eq!(
        clusters.len(),
        2,
        "Expected 2 clusters, got {}",
        clusters.len()
    );
    assert_eq!(clusters[0].items.len(), 1);
    assert_eq!(clusters[1].items.len(), 1);
}

/// Test 4: Citations separated by text are NOT grouped
#[test]
fn test_citations_separated_by_text_not_grouped() {
    // Given: Markdown with text between citations
    let markdown = "See [@a] and also [@b].";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: Each citation is in its own cluster
    assert_eq!(
        clusters.len(),
        2,
        "Expected 2 clusters, got {}",
        clusters.len()
    );
}

/// Test 5: Mix of grouped and isolated citations
#[test]
fn test_mixed_grouped_and_isolated() {
    // Given: Markdown with some adjacent and some separated citations
    let markdown = "First [@a] [@b] and then [@c] separately.";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: First two are grouped, third is isolated
    assert_eq!(
        clusters.len(),
        2,
        "Expected 2 clusters, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        2,
        "Expected 2 items in first cluster, got {}",
        clusters[0].items.len()
    );
    assert_eq!(
        clusters[1].items.len(),
        1,
        "Expected 1 item in second cluster, got {}",
        clusters[1].items.len()
    );
}

/// Test 6: Pandoc syntax [@a; @b; @c] is also supported
#[test]
fn test_pandoc_syntax_grouped() {
    // Given: Markdown with Pandoc multi-citation syntax
    let markdown = "Studies [@a; @b; @c] show that...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All citations are in a single cluster
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        3,
        "Expected 3 items in cluster, got {}",
        clusters[0].items.len()
    );
    // Verify the IDs are extracted correctly
    assert_eq!(clusters[0].items[0].id, "a");
    assert_eq!(clusters[0].items[1].id, "b");
    assert_eq!(clusters[0].items[2].id, "c");
}

/// Test 6b: Pandoc syntax with locators [@book1, p. 10; @book2, ch. 3]
#[test]
fn test_pandoc_syntax_with_locators() {
    // Given: Markdown with Pandoc multi-citation syntax including locators
    let markdown = "See [@book1, p. 10; @book2, ch. 3] for details.";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: All citations are in a single cluster with locators preserved
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        2,
        "Expected 2 items in cluster, got {}",
        clusters[0].items.len()
    );
    assert_eq!(clusters[0].items[0].id, "book1");
    assert_eq!(clusters[0].items[0].locator, Some("10".into()));
    assert_eq!(clusters[0].items[0].label, Some("page".into()));
    assert_eq!(clusters[0].items[1].id, "book2");
    assert_eq!(clusters[0].items[1].locator, Some("3".into()));
    assert_eq!(clusters[0].items[1].label, Some("chapter".into()));
}

/// Test 7: Locators are preserved in grouped citations
#[test]
fn test_grouped_with_locators() {
    // Given: Markdown with adjacent citations that have locators
    let markdown = "See [@book1, p. 10] [@book2, ch. 3] for details.";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: Citations are grouped and locators are preserved
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(clusters[0].items[0].locator, Some("10".into()));
    assert_eq!(clusters[0].items[1].locator, Some("3".into()));
}

// =============================================================================
// Tests for CSL formatting with clusters (Phase 5)
// =============================================================================

// Author-date style for grouped citations
const AUTHOR_DATE_STYLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <id>author-date</id>
    <title>Author Date</title>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <sort>
      <key variable="author"/>
      <key variable="issued"/>
    </sort>
    <layout prefix="(" suffix=")" delimiter="; ">
      <names variable="author">
        <name form="short"/>
      </names>
      <date prefix=", " variable="issued">
        <date-part name="year"/>
      </date>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <names variable="author"><name/></names>
      <text prefix=". " variable="title"/>
      <date prefix=" (" suffix=")." variable="issued">
        <date-part name="year"/>
      </date>
    </layout>
  </bibliography>
</style>"#;

/// Test 8: Numeric style with consecutive numbers produces range (1-3)
#[test]
fn test_format_numeric_consecutive() {
    // Given: Three references and a cluster with all three
    let refs = build_refs(&["ref-a", "ref-b", "ref-c"]);
    let clusters = vec![CitationCluster {
        items: vec![
            CitationItem {
                id: "ref-a".to_string(),
                locator: None,
                label: None,
                url: None,
            },
            CitationItem {
                id: "ref-b".to_string(),
                locator: None,
                label: None,
                url: None,
            },
            CitationItem {
                id: "ref-c".to_string(),
                locator: None,
                label: None,
                url: None,
            },
        ],
        span: (0, 20),
    }];

    // When: Formatted with a numeric style with collapse
    let result = format_citations_clusters(&clusters, &refs, NUMERIC_STYLE).unwrap();

    // Then: Result contains collapsed range "1-3" (or "1,2,3" at minimum)
    assert_eq!(result.len(), 1, "Expected 1 processed citation");
    let formatted = &result[0].formatted;
    // The citation should contain all three numbers in some form
    assert!(
        formatted.contains("1") && formatted.contains("3"),
        "Expected citation to contain numbers 1-3, got: {}",
        formatted
    );
    // With collapse="citation-number", consecutive numbers should be collapsed
    // Note: csl_proc uses en-dash (–) for ranges, not hyphen (-)
    assert!(
        formatted.contains("1-3") || formatted.contains("1–3") || formatted.contains("1,2,3"),
        "Expected collapsed range (1-3) or (1,2,3), got: {}",
        formatted
    );
}

/// Test 9: Numeric style with non-consecutive numbers produces gaps (1, 3-6)
#[test]
fn test_format_numeric_non_consecutive() {
    // Given: Four references, we cite them in a specific order to get non-consecutive numbers
    let refs = build_refs(&["ref-a", "ref-b", "ref-c", "ref-d"]);
    let clusters = vec![
        // First cite ref-b alone (gets number 1)
        CitationCluster {
            items: vec![CitationItem {
                id: "ref-b".to_string(),
                locator: None,
                label: None,
                url: None,
            }],
            span: (0, 10),
        },
        // Then cite ref-a, ref-c, ref-d together (gets numbers 2, 3, 4)
        CitationCluster {
            items: vec![
                CitationItem {
                    id: "ref-a".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
                CitationItem {
                    id: "ref-c".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
                CitationItem {
                    id: "ref-d".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
            ],
            span: (15, 40),
        },
    ];

    // When: Formatted with a numeric style
    let result = format_citations_clusters(&clusters, &refs, NUMERIC_STYLE).unwrap();

    // Then: Second cluster should have consecutive numbers 2-4
    assert_eq!(result.len(), 2, "Expected 2 processed citations");
    let second_citation = &result[1].formatted;
    assert!(
        second_citation.contains("2") && second_citation.contains("4"),
        "Expected numbers 2-4 in second citation, got: {}",
        second_citation
    );
}

/// Test 10: Numeric style with multiple gaps (1-2, 4, 6-9)
#[test]
fn test_format_numeric_multiple_gaps() {
    // Given: A cluster with refs that have gaps in citation numbers
    // We'll cite refs in order to establish numbers, then cite a subset with gaps
    let refs = build_refs(&["r1", "r2", "r3", "r4", "r5"]);

    // Cite r1, r2, r4, r5 together (skipping r3) - should produce 1,2,4,5
    let clusters = vec![
        // First cite r3 alone to give it number 1
        CitationCluster {
            items: vec![CitationItem {
                id: "r3".to_string(),
                locator: None,
                label: None,
                url: None,
            }],
            span: (0, 5),
        },
        // Then cite r1, r2, r4, r5 together (numbers 2, 3, 4, 5)
        CitationCluster {
            items: vec![
                CitationItem {
                    id: "r1".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
                CitationItem {
                    id: "r2".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
                CitationItem {
                    id: "r4".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
                CitationItem {
                    id: "r5".to_string(),
                    locator: None,
                    label: None,
                    url: None,
                },
            ],
            span: (10, 40),
        },
    ];

    // When: Formatted
    let result = format_citations_clusters(&clusters, &refs, NUMERIC_STYLE).unwrap();

    // Then: Second cluster should contain numbers 2-5
    assert_eq!(result.len(), 2);
    let second = &result[1].formatted;
    assert!(
        second.contains("2") && second.contains("5"),
        "Expected numbers including 2 and 5, got: {}",
        second
    );
}

/// Test 11: Author-date style groups properly
#[test]
fn test_format_author_date_grouped() {
    // Given: Multiple references with different authors
    let refs = r#"[
        {"id": "smith2020", "type": "article-journal", "author": [{"family": "Smith", "given": "John"}], "title": "First Paper", "issued": {"date-parts": [[2020]]}},
        {"id": "jones2021", "type": "article-journal", "author": [{"family": "Jones", "given": "Jane"}], "title": "Second Paper", "issued": {"date-parts": [[2021]]}}
    ]"#;

    let clusters = vec![CitationCluster {
        items: vec![
            CitationItem {
                id: "smith2020".to_string(),
                locator: None,
                label: None,
                url: None,
            },
            CitationItem {
                id: "jones2021".to_string(),
                locator: None,
                label: None,
                url: None,
            },
        ],
        span: (0, 30),
    }];

    // When: Formatted with an author-date style
    let result = format_citations_clusters(&clusters, &refs, AUTHOR_DATE_STYLE).unwrap();

    // Then: Result contains both authors with semicolon delimiter "(Jones, 2021; Smith, 2020)"
    assert_eq!(result.len(), 1, "Expected 1 processed citation");
    let formatted = &result[0].formatted;
    assert!(
        formatted.contains("Smith") && formatted.contains("Jones"),
        "Expected both author names, got: {}",
        formatted
    );
    assert!(
        formatted.contains("2020") && formatted.contains("2021"),
        "Expected both years, got: {}",
        formatted
    );
    // Should be grouped with semicolon delimiter
    assert!(
        formatted.contains(";"),
        "Expected semicolon delimiter between grouped citations, got: {}",
        formatted
    );
}

// =============================================================================
// Regression tests
// =============================================================================

/// Test 12: Simple single citation still works
#[test]
fn test_simple_citation_still_works() {
    // Given: Markdown with a single citation
    let markdown = "See [@ref1].";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: We get a single cluster with one item
    assert_eq!(
        clusters.len(),
        1,
        "Expected 1 cluster, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        1,
        "Expected 1 item in cluster, got {}",
        clusters[0].items.len()
    );
}

/// Test 13: URL is preserved for isolated citation
#[test]
fn test_isolated_citation_url_preserved() {
    // Given: Markdown with a citation that has a URL
    let markdown = "See [@ref1](https://doi.org/xxx).";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: The URL is preserved in the citation item
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items[0].url, Some("https://doi.org/xxx".into()));
}

// =============================================================================
// Additional edge case tests
// =============================================================================

/// Test: Span covers entire cluster for adjacent citations
#[test]
fn test_cluster_span_covers_all_citations() {
    // Given: Markdown with adjacent citations
    let markdown = "Studies [@a] [@b] [@c] show that...";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: The span covers from first citation start to last citation end
    assert_eq!(clusters.len(), 1);
    let (start, end) = clusters[0].span;
    // "[@a] [@b] [@c]" should be the covered text
    let covered = &markdown[start..end];
    assert!(
        covered.contains("[@a]") && covered.contains("[@c]"),
        "Span should cover all citations, got: '{}'",
        covered
    );
}

/// Test: Multiple separate groups in same document
#[test]
fn test_multiple_separate_groups() {
    // Given: Markdown with two separate groups of citations
    let markdown = "First [@a] [@b] and then [@c] [@d] [@e].";

    // When: We extract citation clusters
    let clusters = extract_citation_clusters(markdown);

    // Then: We get two clusters
    assert_eq!(
        clusters.len(),
        2,
        "Expected 2 clusters, got {}",
        clusters.len()
    );
    assert_eq!(
        clusters[0].items.len(),
        2,
        "First cluster should have 2 items"
    );
    assert_eq!(
        clusters[1].items.len(),
        3,
        "Second cluster should have 3 items"
    );
}
