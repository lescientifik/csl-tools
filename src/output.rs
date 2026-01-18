//! Output generation for formatted citations and bibliographies.
//!
//! This module handles replacing citations in the original Markdown text
//! and generating the final output with the bibliography.

use crate::processor::ProcessedCitation;

/// Replaces citation markers in the Markdown with formatted citations.
///
/// # Arguments
///
/// * `markdown` - The original Markdown text
/// * `processed` - The processed citations with their spans and formatted text
///
/// # Returns
///
/// The Markdown text with citations replaced.
///
/// # Implementation Note
///
/// Replacements are performed from the end of the text towards the beginning
/// to preserve the validity of span indices. This ensures that replacing
/// earlier citations doesn't invalidate the spans of later ones.
pub fn replace_citations(markdown: &str, processed: &[ProcessedCitation]) -> String {
    // Handle empty case
    if processed.is_empty() {
        return markdown.to_string();
    }

    // Create a vector of citations sorted by span start position in descending order
    // This allows us to replace from end to beginning, preserving indices
    let mut sorted_citations: Vec<_> = processed.iter().collect();
    sorted_citations.sort_by(|a, b| b.original_span.0.cmp(&a.original_span.0));

    let mut result = markdown.to_string();

    for citation in sorted_citations {
        let (start, end) = citation.original_span;
        // Replace the span with the formatted citation
        result.replace_range(start..end, &citation.formatted);
    }

    result
}

/// Generates the final output with formatted citations and bibliography.
///
/// # Arguments
///
/// * `content` - The Markdown content with citations already replaced
/// * `bibliography` - The formatted bibliography HTML (if any)
/// * `bib_header` - The header to use for the bibliography section
///
/// # Returns
///
/// The complete output document.
pub fn generate_output(content: &str, bibliography: Option<&str>, bib_header: &str) -> String {
    let mut output = content.trim_end().to_string();

    if let Some(bib) = bibliography {
        if !bib.is_empty() {
            output.push_str("\n\n");
            output.push_str(bib_header);
            output.push_str("\n\n");
            output.push_str(bib);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // Tests for replace_citations (Phase 5.1)
    // ===========================================

    #[test]
    fn test_replace_citations_simple() {
        // Given: A markdown text with one citation marker
        let markdown = "Voir [@item-1] pour details.";
        let processed = vec![ProcessedCitation {
            original_span: (5, 14), // "[@item-1]"
            formatted: "(Doe, 2021)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The citation marker is replaced with formatted text
        assert_eq!(result, "Voir (Doe, 2021) pour details.");
    }

    #[test]
    fn test_replace_citations_multiple() {
        // Given: A markdown text with multiple citations
        let markdown = "First [@a] and second [@b] here.";
        let processed = vec![
            ProcessedCitation {
                original_span: (6, 10), // "[@a]"
                formatted: "(A, 2020)".to_string(),
            },
            ProcessedCitation {
                original_span: (22, 26), // "[@b]"
                formatted: "(B, 2021)".to_string(),
            },
        ];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: All citation markers are replaced
        assert_eq!(result, "First (A, 2020) and second (B, 2021) here.");
    }

    #[test]
    fn test_replace_citations_preserves_markdown() {
        // Given: A markdown text with formatting and a citation
        let markdown = "# Title\n\n**Bold** text with [@cite] and _italic_.";
        let processed = vec![ProcessedCitation {
            original_span: (28, 35), // "[@cite]" - starts after "with "
            formatted: "(Smith, 2019)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: Markdown formatting is preserved
        assert_eq!(
            result,
            "# Title\n\n**Bold** text with (Smith, 2019) and _italic_."
        );
    }

    #[test]
    fn test_replace_citations_empty_list() {
        // Given: A markdown text with no processed citations
        let markdown = "Text without citations.";
        let processed: Vec<ProcessedCitation> = vec![];

        // When: We replace citations (with empty list)
        let result = replace_citations(markdown, &processed);

        // Then: The original text is returned unchanged
        assert_eq!(result, "Text without citations.");
    }

    #[test]
    fn test_replace_citations_with_url() {
        // Given: A citation marker that includes a URL (full span includes URL part)
        let markdown = "See [@item](https://doi.org/10.1234) for more.";
        let processed = vec![ProcessedCitation {
            original_span: (4, 36), // "[@item](https://doi.org/10.1234)"
            formatted: "(Doe, 2021)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The entire citation marker (with URL) is replaced
        assert_eq!(result, "See (Doe, 2021) for more.");
    }

    #[test]
    fn test_replace_citations_at_start() {
        // Given: A citation at the very start of the text
        let markdown = "[@ref] is important.";
        let processed = vec![ProcessedCitation {
            original_span: (0, 6), // "[@ref]"
            formatted: "(Author, 2020)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The citation is correctly replaced
        assert_eq!(result, "(Author, 2020) is important.");
    }

    #[test]
    fn test_replace_citations_at_end() {
        // Given: A citation at the very end of the text
        let markdown = "See the reference [@ref]";
        let processed = vec![ProcessedCitation {
            original_span: (18, 24), // "[@ref]"
            formatted: "(Author, 2020)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The citation is correctly replaced
        assert_eq!(result, "See the reference (Author, 2020)");
    }

    #[test]
    fn test_replace_citations_longer_replacement() {
        // Given: A short citation marker replaced with longer text
        let markdown = "Text [@a] more.";
        let processed = vec![ProcessedCitation {
            original_span: (5, 9), // "[@a]"
            formatted: "(Very Long Author Name, 2021, pp. 100-200)".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The replacement works even when the new text is longer
        assert_eq!(
            result,
            "Text (Very Long Author Name, 2021, pp. 100-200) more."
        );
    }

    #[test]
    fn test_replace_citations_shorter_replacement() {
        // Given: A long citation marker replaced with shorter text
        let markdown = "Text [@very-long-citation-key] more.";
        let processed = vec![ProcessedCitation {
            original_span: (5, 30), // "[@very-long-citation-key]"
            formatted: "[1]".to_string(),
        }];

        // When: We replace citations
        let result = replace_citations(markdown, &processed);

        // Then: The replacement works even when the new text is shorter
        assert_eq!(result, "Text [1] more.");
    }

    // ===========================================
    // Tests for generate_output (Phase 5.2)
    // ===========================================

    #[test]
    fn test_generate_output_no_bib() {
        let result = generate_output("Some text", None, "## References");
        assert_eq!(result, "Some text");
    }

    #[test]
    fn test_generate_output_with_bib() {
        let result = generate_output("Some text", Some("<div>Bib</div>"), "## References");
        assert!(result.contains("## References"));
        assert!(result.contains("<div>Bib</div>"));
    }
}
