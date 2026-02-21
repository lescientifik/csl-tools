//! Citation and bibliography processing using csl_proc.
//!
//! This module orchestrates the formatting of citations and bibliographies
//! by calling into the csl_proc library.

use crate::markdown::Citation;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during processing.
#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("CSL processing error: {0}")]
    CslError(String),

    #[error("Reference not found: {0}")]
    ReferenceNotFound(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

}

/// A citation that has been formatted by csl_proc.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessedCitation {
    /// The span in the original text where this citation was found
    pub original_span: (usize, usize),
    /// The formatted citation text (e.g., "(Doe, 2021)")
    pub formatted: String,
}

/// Formats citations using the given references and style.
///
/// # Arguments
///
/// * `citations` - The citations extracted from the Markdown
/// * `refs_json` - The CSL-JSON references as a string
/// * `style_csl` - The CSL style XML as a string
///
/// # Returns
///
/// A vector of formatted citations in the same order as input.
pub fn format_citations(
    citations: &[Citation],
    refs_json: &str,
    style_csl: &str,
) -> Result<Vec<ProcessedCitation>, ProcessorError> {
    // Handle empty citations case early
    if citations.is_empty() {
        return Ok(Vec::new());
    }

    // Validate JSON first
    let refs_array: Value =
        serde_json::from_str(refs_json).map_err(|e| ProcessorError::InvalidJson(e.to_string()))?;

    // Ensure refs is an array
    let refs_array = refs_array.as_array().ok_or_else(|| {
        ProcessorError::InvalidJson("References must be a JSON array".to_string())
    })?;

    // Build a set of available reference IDs for validation
    let available_ids: HashSet<String> = refs_array
        .iter()
        .filter_map(|r| r.get("id").and_then(|id| id.as_str()))
        .map(|s| s.to_string())
        .collect();

    // Verify all cited references exist
    for citation in citations {
        if !available_ids.contains(&citation.id) {
            return Err(ProcessorError::ReferenceNotFound(citation.id.clone()));
        }
    }

    // Build citation_items JSON for csl_proc
    // Each citation gets its own cluster for individual formatting
    let citation_items: Vec<Vec<serde_json::Value>> = citations
        .iter()
        .map(|c| {
            let mut item = serde_json::json!({"id": c.id});
            // Add locator if present
            if let Some(ref locator) = c.locator {
                item["locator"] = serde_json::json!(locator);
            }
            if let Some(ref label) = c.label {
                item["label"] = serde_json::json!(label);
            }
            vec![item]
        })
        .collect();

    let citation_items_json = serde_json::to_string(&citation_items)
        .map_err(|e| ProcessorError::CslError(e.to_string()))?;

    // Call csl_proc to format citations
    let formatted_output = csl_proc::process_with_citations(
        style_csl,
        refs_json,
        "citation",
        Some(&citation_items_json),
    )
    .map_err(ProcessorError::CslError)?;

    // Parse the output - csl_proc returns one line per citation cluster
    let formatted_lines: Vec<&str> = formatted_output.lines().collect();

    // Build ProcessedCitation for each input citation
    let mut result = Vec::with_capacity(citations.len());
    for (i, citation) in citations.iter().enumerate() {
        let formatted = formatted_lines
            .get(i)
            .map(|s| s.to_string())
            .unwrap_or_default();
        result.push(ProcessedCitation {
            original_span: citation.span,
            formatted,
        });
    }

    Ok(result)
}

/// Formats citation clusters using the given references and style.
///
/// This function handles grouped citations properly, where multiple citations
/// in a single cluster are formatted together (e.g., "(1-3)" instead of "(1) (2) (3)").
///
/// # Arguments
///
/// * `clusters` - The citation clusters extracted from the Markdown
/// * `refs_json` - The CSL-JSON references as a string
/// * `style_csl` - The CSL style XML as a string
///
/// # Returns
///
/// A vector of formatted citations, one per cluster.
pub fn format_citations_clusters(
    clusters: &[crate::markdown::CitationCluster],
    refs_json: &str,
    style_csl: &str,
) -> Result<Vec<ProcessedCitation>, ProcessorError> {
    // Handle empty clusters case early
    if clusters.is_empty() {
        return Ok(Vec::new());
    }

    // Validate JSON first
    let refs_array: Value =
        serde_json::from_str(refs_json).map_err(|e| ProcessorError::InvalidJson(e.to_string()))?;

    // Ensure refs is an array
    let refs_array = refs_array.as_array().ok_or_else(|| {
        ProcessorError::InvalidJson("References must be a JSON array".to_string())
    })?;

    // Build a set of available reference IDs for validation
    let available_ids: HashSet<String> = refs_array
        .iter()
        .filter_map(|r| r.get("id").and_then(|id| id.as_str()))
        .map(|s| s.to_string())
        .collect();

    // Verify all cited references exist
    for cluster in clusters {
        for item in &cluster.items {
            if !available_ids.contains(&item.id) {
                return Err(ProcessorError::ReferenceNotFound(item.id.clone()));
            }
        }
    }

    // Build citation_items JSON for csl_proc
    // Each cluster becomes an array of items (for grouping)
    let citation_items: Vec<Vec<serde_json::Value>> = clusters
        .iter()
        .map(|cluster| {
            cluster
                .items
                .iter()
                .map(|item| {
                    let mut json_item = serde_json::json!({"id": item.id});
                    // Add locator if present
                    if let Some(ref locator) = item.locator {
                        json_item["locator"] = serde_json::json!(locator);
                    }
                    if let Some(ref label) = item.label {
                        json_item["label"] = serde_json::json!(label);
                    }
                    json_item
                })
                .collect()
        })
        .collect();

    let citation_items_json = serde_json::to_string(&citation_items)
        .map_err(|e| ProcessorError::CslError(e.to_string()))?;

    // Call csl_proc to format citations
    let formatted_output = csl_proc::process_with_citations(
        style_csl,
        refs_json,
        "citation",
        Some(&citation_items_json),
    )
    .map_err(ProcessorError::CslError)?;

    // Parse the output - csl_proc returns one line per citation cluster
    let formatted_lines: Vec<&str> = formatted_output.lines().collect();

    // Build ProcessedCitation for each input cluster
    let mut result = Vec::with_capacity(clusters.len());
    for (i, cluster) in clusters.iter().enumerate() {
        let formatted = formatted_lines
            .get(i)
            .map(|s| s.to_string())
            .unwrap_or_default();
        result.push(ProcessedCitation {
            original_span: cluster.span,
            formatted,
        });
    }

    Ok(result)
}

/// Formats the bibliography for the cited references.
///
/// # Arguments
///
/// * `citations` - The citations to include in the bibliography
/// * `refs_json` - The CSL-JSON references as a string
/// * `style_csl` - The CSL style XML as a string
///
/// # Returns
///
/// The formatted bibliography as HTML.
pub fn format_bibliography(
    citations: &[Citation],
    refs_json: &str,
    style_csl: &str,
) -> Result<String, ProcessorError> {
    // Handle empty citations case early
    if citations.is_empty() {
        return Ok(String::new());
    }

    // Parse the references JSON
    let all_refs: Value =
        serde_json::from_str(refs_json).map_err(|e| ProcessorError::InvalidJson(e.to_string()))?;

    // Ensure refs is an array
    let all_refs = all_refs.as_array().ok_or_else(|| {
        ProcessorError::InvalidJson("References must be a JSON array".to_string())
    })?;

    // Build refs index for O(1) lookup
    let refs_by_id: HashMap<&str, &Value> = all_refs
        .iter()
        .filter_map(|r| r.get("id").and_then(|id| id.as_str()).map(|id| (id, r)))
        .collect();

    // Order refs by first appearance in text (citations is already in document order).
    // For styles WITHOUT <sort> in <bibliography>, csl_proc assigns citation-number = i+1
    // based on array position, so this ordering determines the final bibliography order.
    // For styles WITH <sort> in <bibliography>, csl_proc re-sorts entries anyway — our
    // ordering is a sensible default that gets overridden by the style.
    //
    // Note: missing refs are silently skipped here. In practice format_citations_clusters()
    // runs first in main.rs and returns ReferenceNotFound, so this path is only reachable
    // if format_bibliography() is called directly via the public API.
    let mut seen = HashSet::new();
    let mut cited_refs: Vec<&Value> = Vec::new();
    for citation in citations {
        if seen.insert(citation.id.as_str()) {
            if let Some(&ref_item) = refs_by_id.get(citation.id.as_str()) {
                cited_refs.push(ref_item);
            }
        }
    }

    // If no cited references found, return empty
    if cited_refs.is_empty() {
        return Ok(String::new());
    }

    // Convert filtered refs back to JSON string
    let filtered_refs_json =
        serde_json::to_string(&cited_refs).map_err(|e| ProcessorError::CslError(e.to_string()))?;

    // Call csl_proc to format bibliography
    let bibliography_output = csl_proc::process(style_csl, &filtered_refs_json, "bibliography")
        .map_err(ProcessorError::CslError)?;

    Ok(bibliography_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal CSL style for testing
    const MINIMAL_STYLE: &str = r#"<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <id>test-style</id>
    <title>Test Style</title>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <layout prefix="(" suffix=")" delimiter="; ">
      <names variable="author">
        <name form="short"/>
      </names>
      <text prefix=", " variable="issued" date-parts="year"/>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <names variable="author"/>
      <text prefix=". " variable="title" font-style="italic"/>
      <text prefix=" (" suffix=")." variable="issued" date-parts="year"/>
    </layout>
  </bibliography>
</style>"#;

    // ===========================================
    // Tests for format_citations (Phase 4.1)
    // ===========================================

    #[test]
    fn test_format_citations_single() {
        // Given: A single citation and matching reference
        let citations = vec![Citation {
            id: "item-1".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (10, 20),
        }];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: We get one formatted citation with correct span and text
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].original_span, (10, 20));
        assert!(
            result[0].formatted.contains("Doe"),
            "Expected 'Doe' in formatted citation, got: {}",
            result[0].formatted
        );
        // Note: Year formatting depends on CSL style implementation
        // The key test is that we get the author name correctly
        assert!(
            !result[0].formatted.is_empty(),
            "Expected non-empty formatted citation"
        );
    }

    #[test]
    fn test_format_citations_multiple() {
        // Given: Multiple citations with matching references
        let citations = vec![
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (5, 15),
            },
            Citation {
                id: "item-2".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (30, 40),
            },
        ];
        let refs = r#"[
            {"id": "item-1", "type": "book", "author": [{"family": "Smith", "given": "Alice"}], "title": "First Book", "issued": {"date-parts": [[2020]]}},
            {"id": "item-2", "type": "book", "author": [{"family": "Jones", "given": "Bob"}], "title": "Second Book", "issued": {"date-parts": [[2021]]}}
        ]"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: We get two formatted citations in order
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].original_span, (5, 15));
        assert!(result[0].formatted.contains("Smith"));
        assert_eq!(result[1].original_span, (30, 40));
        assert!(result[1].formatted.contains("Jones"));
    }

    #[test]
    fn test_format_citations_missing_reference() {
        // Given: A citation with no matching reference
        let citations = vec![Citation {
            id: "nonexistent".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 15),
        }];
        let refs =
            r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe"}], "title": "Book"}]"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE);

        // Then: We get an error about the missing reference
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("not found") || error_msg.contains("nonexistent"),
            "Expected error about missing reference, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_format_citations_invalid_json() {
        // Given: Invalid JSON references
        let citations = vec![Citation {
            id: "item-1".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 10),
        }];
        let refs = r#"[{"id": "item-1", "invalid json"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE);

        // Then: We get an error about invalid JSON
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.to_lowercase().contains("json") || error_msg.to_lowercase().contains("parse"),
            "Expected JSON parsing error, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_format_citations_empty_list() {
        // Given: An empty list of citations
        let citations: Vec<Citation> = vec![];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe"}]}]"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: We get an empty result
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_citations_duplicate_citation() {
        // Given: The same citation appearing twice
        let citations = vec![
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (5, 15),
            },
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (30, 40),
            },
        ];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]"#;

        // When: We format citations
        let result = format_citations(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: Both citations are formatted (same reference cited twice)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].original_span, (5, 15));
        assert_eq!(result[1].original_span, (30, 40));
        // Both should have similar formatting (same author)
        assert!(result[0].formatted.contains("Doe"));
        assert!(result[1].formatted.contains("Doe"));
    }

    // ===========================================
    // Tests for format_bibliography (Phase 4.2)
    // ===========================================

    #[test]
    fn test_format_bibliography_single() {
        // Given: A single citation
        let citations = vec![Citation {
            id: "item-1".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 10),
        }];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]"#;

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: We get HTML with the formatted entry
        assert!(
            result.contains("csl-bib-body") || result.contains("csl-entry"),
            "Expected CSL HTML structure, got: {}",
            result
        );
        assert!(
            result.contains("Doe") || result.contains("John"),
            "Expected author name in bibliography, got: {}",
            result
        );
        assert!(
            result.contains("Test Book"),
            "Expected title in bibliography, got: {}",
            result
        );
    }

    #[test]
    fn test_format_bibliography_multiple() {
        // Given: Multiple citations
        let citations = vec![
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (0, 10),
            },
            Citation {
                id: "item-2".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (20, 30),
            },
        ];
        let refs = r#"[
            {"id": "item-1", "type": "book", "author": [{"family": "Smith", "given": "Alice"}], "title": "First Book", "issued": {"date-parts": [[2020]]}},
            {"id": "item-2", "type": "book", "author": [{"family": "Jones", "given": "Bob"}], "title": "Second Book", "issued": {"date-parts": [[2021]]}}
        ]"#;

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: Both entries appear in the bibliography
        assert!(result.contains("Smith") || result.contains("Alice"));
        assert!(result.contains("Jones") || result.contains("Bob"));
        assert!(result.contains("First Book"));
        assert!(result.contains("Second Book"));
    }

    #[test]
    fn test_format_bibliography_deduplicates() {
        // Given: Same citation cited twice
        let citations = vec![
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (0, 10),
            },
            Citation {
                id: "item-1".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (20, 30),
            },
        ];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]"#;

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: Only one entry appears in the bibliography
        let entry_count = result.matches("csl-entry").count();
        assert_eq!(
            entry_count, 1,
            "Expected 1 bibliography entry, got {} in: {}",
            entry_count, result
        );
    }

    #[test]
    fn test_format_bibliography_empty_citations() {
        // Given: No citations
        let citations: Vec<Citation> = vec![];
        let refs = r#"[{"id": "item-1", "type": "book", "author": [{"family": "Doe"}]}]"#;

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: We get an empty bibliography
        assert!(
            result.is_empty() || !result.contains("csl-entry"),
            "Expected empty bibliography, got: {}",
            result
        );
    }

    // Numeric CSL style WITHOUT <sort> in <bibliography> — bibliography order
    // depends entirely on the order of the refs array passed to csl_proc.
    // NOTE: Same CSL content as `tests/common::NUMERIC_STYLE` (duplicated here
    // because unit tests cannot import from integration test crates).
    const NUMERIC_NOSORT_STYLE: &str = r#"<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <id>numeric-nosort</id>
    <title>Numeric No Sort</title>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation collapse="citation-number">
    <sort><key variable="citation-number"/></sort>
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

    #[test]
    fn test_format_bibliography_order_matches_citation_order() {
        // Given: Refs JSON in order [Bravo, Alpha], citations reference [Alpha, Bravo]
        let refs = r#"[
            {"id": "bravo", "type": "article-journal", "author": [{"family": "Bravo", "given": "B."}], "title": "Bravo Title", "issued": {"date-parts": [[2020]]}},
            {"id": "alpha", "type": "article-journal", "author": [{"family": "Alpha", "given": "A."}], "title": "Alpha Title", "issued": {"date-parts": [[2021]]}}
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
        ];

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, NUMERIC_NOSORT_STYLE).unwrap();

        // Then: Alpha (cited first) should appear before Bravo in the bibliography
        let alpha_pos = result.find("Alpha").expect("Alpha should appear in bibliography");
        let bravo_pos = result.find("Bravo").expect("Bravo should appear in bibliography");
        assert!(
            alpha_pos < bravo_pos,
            "Alpha (cited first) should appear before Bravo (cited second) in bibliography. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_format_bibliography_dedup_preserves_first_appearance() {
        // Given: Ref A cited, then ref B, then ref A again
        let refs = r#"[
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
                id: "alpha".to_string(),
                locator: None,
                label: None,
                url: None,
                span: (40, 50),
            },
        ];

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, NUMERIC_NOSORT_STYLE).unwrap();

        // Then: Alpha=1, Bravo=2, and only 2 entries (no duplicate Alpha)
        let entry_count = result.matches("csl-entry").count();
        assert_eq!(
            entry_count, 2,
            "Expected 2 bibliography entries (no duplicates), got {} in:\n{}",
            entry_count, result
        );
        let alpha_pos = result.find("Alpha").expect("Alpha should appear in bibliography");
        let bravo_pos = result.find("Bravo").expect("Bravo should appear in bibliography");
        assert!(
            alpha_pos < bravo_pos,
            "Alpha (first appearance) should come before Bravo in bibliography. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_format_bibliography_only_cited_refs() {
        // Given: Citations for only one of two available references
        let citations = vec![Citation {
            id: "item-1".to_string(),
            locator: None,
            label: None,
            url: None,
            span: (0, 10),
        }];
        let refs = r#"[
            {"id": "item-1", "type": "book", "author": [{"family": "Doe"}], "title": "Cited Book"},
            {"id": "item-2", "type": "book", "author": [{"family": "Smith"}], "title": "Uncited Book"}
        ]"#;

        // When: We format the bibliography
        let result = format_bibliography(&citations, refs, MINIMAL_STYLE).unwrap();

        // Then: Only the cited reference appears
        assert!(
            result.contains("Cited Book"),
            "Expected cited book in bibliography, got: {}",
            result
        );
        assert!(
            !result.contains("Uncited Book"),
            "Expected uncited book NOT in bibliography, got: {}",
            result
        );
    }
}
