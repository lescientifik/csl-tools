//! Markdown citation parser.
//!
//! Extracts citations in the format `[@id]`, `[@id](url)`, and `[@id, p. 42]`
//! from Markdown text.
//!
//! Also supports citation clustering for adjacent citations and Pandoc syntax.

use regex::Regex;

/// Extracts Pandoc-style grouped citations like `[@a; @b; @c]` or `[@a, p. 10; @b, ch. 3]`.
///
/// This function finds citations in the Pandoc multi-citation syntax where multiple
/// citation items are separated by semicolons within a single bracket pair.
///
/// # Returns
///
/// A vector of `CitationCluster` structs, each containing multiple `CitationItem`s.
fn extract_pandoc_grouped_citations(markdown: &str) -> Vec<CitationCluster> {
    // Regex to match Pandoc grouped citations: [@id1; @id2; @id3] or [@id1, locator; @id2]
    // This matches brackets containing multiple @-prefixed citations separated by semicolons
    let pandoc_re = Regex::new(r"\[(@[^\]]+;[^\]]*)\]").unwrap();

    let mut clusters: Vec<CitationCluster> = Vec::new();

    for cap in pandoc_re.captures_iter(markdown) {
        let full_match = cap.get(0).unwrap();
        let inner = cap.get(1).unwrap().as_str();

        // Split by semicolon and parse each citation item
        let mut items: Vec<CitationItem> = Vec::new();

        for part in inner.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Each part should start with @ and may have a locator after comma
            // Format: @id or @id, locator
            if let Some(stripped) = part.strip_prefix('@') {
                // Check if there's a locator (comma-separated)
                let (id, locator, label) = if let Some(comma_pos) = stripped.find(',') {
                    let id = stripped[..comma_pos].trim().to_string();
                    let locator_str = stripped[comma_pos + 1..].trim();
                    let (locator, label) = parse_locator(locator_str);
                    (id, locator, label)
                } else {
                    (stripped.trim().to_string(), None, None)
                };

                items.push(CitationItem {
                    id,
                    locator,
                    label,
                    url: None, // Pandoc syntax doesn't support URLs
                });
            }
        }

        if !items.is_empty() {
            clusters.push(CitationCluster {
                items,
                span: (full_match.start(), full_match.end()),
            });
        }
    }

    clusters
}

/// An individual citation element (a single @id).
///
/// This structure represents a single citation item within a cluster.
/// Multiple `CitationItem`s can be grouped together in a `CitationCluster`.
#[derive(Debug, Clone, PartialEq)]
pub struct CitationItem {
    /// The citation key (e.g., "item-1" or "pmid:12345")
    pub id: String,
    /// Optional locator value (e.g., "42" for page 42)
    pub locator: Option<String>,
    /// Optional locator label (e.g., "page", "chapter")
    pub label: Option<String>,
    /// Optional URL associated with the citation (preserved for reference, ignored in grouped rendering)
    pub url: Option<String>,
}

/// A group of citations (one or more items in a single cluster).
///
/// Adjacent citations in Markdown (separated only by whitespace) are grouped
/// into a single cluster for proper CSL formatting (e.g., "(1-3)" instead of "(1) (2) (3)").
#[derive(Debug, Clone, PartialEq)]
pub struct CitationCluster {
    /// The citation items in this cluster
    pub items: Vec<CitationItem>,
    /// Start and end byte positions covering the entire cluster in the source text
    pub span: (usize, usize),
}

/// Extracts citation clusters from the given Markdown text.
///
/// This function detects adjacent citations (separated only by whitespace)
/// and groups them into clusters. It also supports Pandoc syntax `[@a; @b; @c]`.
///
/// # Arguments
///
/// * `markdown` - The Markdown text to parse
///
/// # Returns
///
/// A vector of `CitationCluster` structs representing all citation clusters found.
///
/// # Examples
///
/// ```
/// use csl_tools::extract_citation_clusters;
///
/// // Adjacent citations are grouped
/// let clusters = extract_citation_clusters("Studies [@a] [@b] show that...");
/// assert_eq!(clusters.len(), 1);
/// assert_eq!(clusters[0].items.len(), 2);
///
/// // Citations separated by text are not grouped
/// let clusters = extract_citation_clusters("See [@a] and also [@b].");
/// assert_eq!(clusters.len(), 2);
/// ```
pub fn extract_citation_clusters(markdown: &str) -> Vec<CitationCluster> {
    // First, extract Pandoc-style grouped citations [@a; @b; @c]
    let pandoc_clusters = extract_pandoc_grouped_citations(markdown);

    // Collect spans covered by Pandoc citations to avoid duplicates
    let pandoc_spans: Vec<(usize, usize)> = pandoc_clusters.iter().map(|c| c.span).collect();

    // Helper function to check if a position is inside a Pandoc citation
    let is_inside_pandoc = |pos: usize| -> bool {
        pandoc_spans
            .iter()
            .any(|(start, end)| pos >= *start && pos < *end)
    };

    // Extract all individual citations using the existing function
    let citations = extract_citations(markdown);

    // Filter out citations that are inside Pandoc grouped citations
    let simple_citations: Vec<Citation> = citations
        .into_iter()
        .filter(|c| !is_inside_pandoc(c.span.0))
        .collect();

    // Group adjacent simple citations into clusters
    let mut simple_clusters: Vec<CitationCluster> = Vec::new();
    let mut current_items: Vec<CitationItem> = Vec::new();
    let mut cluster_start: usize = 0;
    let mut last_end: usize = 0;

    for citation in simple_citations {
        let item = CitationItem {
            id: citation.id,
            locator: citation.locator,
            label: citation.label,
            url: citation.url,
        };

        if current_items.is_empty() {
            // First item in a potential cluster
            cluster_start = citation.span.0;
            last_end = citation.span.1;
            current_items.push(item);
        } else {
            // Check if this citation is adjacent to the previous one
            // "Adjacent" means only whitespace (spaces, tabs) between them
            let between = &markdown[last_end..citation.span.0];
            let is_adjacent = between.chars().all(|c| c == ' ' || c == '\t');

            if is_adjacent {
                // Add to current cluster
                last_end = citation.span.1;
                current_items.push(item);
            } else {
                // Start a new cluster - save the current one first
                simple_clusters.push(CitationCluster {
                    items: current_items,
                    span: (cluster_start, last_end),
                });

                // Start new cluster
                current_items = vec![item];
                cluster_start = citation.span.0;
                last_end = citation.span.1;
            }
        }
    }

    // Don't forget the last cluster
    if !current_items.is_empty() {
        simple_clusters.push(CitationCluster {
            items: current_items,
            span: (cluster_start, last_end),
        });
    }

    // Merge Pandoc clusters and simple clusters, sorted by position
    let mut all_clusters: Vec<CitationCluster> = pandoc_clusters;
    all_clusters.extend(simple_clusters);
    all_clusters.sort_by_key(|c| c.span.0);

    all_clusters
}

/// Represents a citation found in the Markdown text.
#[derive(Debug, Clone, PartialEq)]
pub struct Citation {
    /// The citation key (e.g., "item-1" or "pmid:12345")
    pub id: String,
    /// Optional locator value (e.g., "42" for page 42)
    pub locator: Option<String>,
    /// Optional locator label (e.g., "page", "chapter")
    pub label: Option<String>,
    /// Optional URL associated with the citation
    pub url: Option<String>,
    /// Start and end byte positions in the original text
    pub span: (usize, usize),
}

/// Extracts all citations from the given Markdown text.
///
/// # Arguments
///
/// * `markdown` - The Markdown text to parse
///
/// # Returns
///
/// A vector of `Citation` structs representing all citations found.
///
/// # Examples
///
/// ```
/// use csl_tools::extract_citations;
///
/// let citations = extract_citations("See [@item-1] for details.");
/// assert_eq!(citations.len(), 1);
/// assert_eq!(citations[0].id, "item-1");
/// ```
pub fn extract_citations(markdown: &str) -> Vec<Citation> {
    // Regex for citation: [@id], [@id, locator], [@id](url), or [@id, locator](url)
    // Group 1: id (required)
    // Group 2: locator part after comma (optional)
    // Group 3: url (optional)
    let re = Regex::new(r"\[@([^\]\[,]+)(?:,\s*([^\]]+))?\](?:\(([^)]+)\))?").unwrap();

    re.captures_iter(markdown)
        .map(|cap| {
            let full_match = cap.get(0).unwrap();
            let id = cap.get(1).unwrap().as_str().trim().to_string();

            // Parse the optional locator part
            let (locator, label) = if let Some(locator_match) = cap.get(2) {
                parse_locator(locator_match.as_str())
            } else {
                (None, None)
            };

            // Parse the optional URL
            let url = cap.get(3).map(|m| m.as_str().to_string());

            Citation {
                id,
                locator,
                label,
                url,
                span: (full_match.start(), full_match.end()),
            }
        })
        .collect()
}

/// Parses a locator string like "p. 42", "pp. 10-20", "ch. 3", "sec. 4.2"
/// or full labels like "page 15", "pages 5-10", "chapter 7", "section 2.1"
///
/// Returns (locator_value, label) tuple.
fn parse_locator(locator_str: &str) -> (Option<String>, Option<String>) {
    let locator_str = locator_str.trim();

    // Define patterns for different locator types
    // Order matters: check longer prefixes before shorter ones to avoid partial matches
    let patterns = [
        // Abbreviations (pp. before p.)
        ("pp.", "page"),
        ("p.", "page"),
        ("ch.", "chapter"),
        ("sec.", "section"),
        // Full words (pages before page)
        ("pages", "page"),
        ("page", "page"),
        ("chapter", "chapter"),
        ("section", "section"),
    ];

    for (prefix, label) in patterns {
        if let Some(stripped) = locator_str.strip_prefix(prefix) {
            let value = stripped.trim().to_string();
            if !value.is_empty() {
                return (Some(value), Some(label.to_string()));
            }
        }
    }

    // If no recognized label, return the raw locator with no label
    if !locator_str.is_empty() {
        (Some(locator_str.to_string()), None)
    } else {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let citations = extract_citations("");
        assert!(citations.is_empty());
    }

    #[test]
    fn test_no_citations() {
        let citations = extract_citations("This is plain text without citations.");
        assert!(citations.is_empty());
    }

    // Sub-phase 2.1: Basic citation parsing tests

    #[test]
    fn test_simple_citation() {
        // Given: Markdown with a simple citation
        let markdown = "Les résultats montrent [@item-1] que la méthode fonctionne.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find one citation with the correct id
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[0].locator, None);
        assert_eq!(citations[0].label, None);
        assert_eq!(citations[0].url, None);
    }

    #[test]
    fn test_pmid_format_citation() {
        // Given: Markdown with a PMID-prefixed citation key
        let markdown = "L'étude [@pmid:12345678] a démontré ces résultats.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find one citation with the PMID key
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "pmid:12345678");
    }

    #[test]
    fn test_citation_span() {
        // Given: Markdown with a citation
        let markdown = "Text [@item-1] more text.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: The span correctly points to the citation in the text
        assert_eq!(citations.len(), 1);
        let (start, end) = citations[0].span;
        assert_eq!(&markdown[start..end], "[@item-1]");
    }

    #[test]
    fn test_multiple_citations() {
        // Given: Markdown with multiple citations
        let markdown = "First [@item-1] and second [@item-2] citations.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find both citations in order
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[1].id, "item-2");
    }

    #[test]
    fn test_citation_with_special_chars() {
        // Given: Markdown with citation keys containing special characters
        let markdown = "See [@doi:10.1234/test_key] for details.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: The key with special characters is extracted correctly
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "doi:10.1234/test_key");
    }

    #[test]
    fn test_citation_at_start_of_text() {
        // Given: Citation at the very start
        let markdown = "[@item-1] begins the sentence.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: Citation is found
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[0].span.0, 0);
    }

    #[test]
    fn test_citation_at_end_of_text() {
        // Given: Citation at the very end
        let markdown = "The source is [@item-1]";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: Citation is found
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "item-1");
    }

    // Sub-phase 2.2: Citation with URL parsing tests

    #[test]
    fn test_citation_with_doi_url() {
        // Given: Markdown with a citation followed by a DOI URL
        let markdown =
            "Les coronavirus [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) sont étudiés.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find one citation with the correct id and URL
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "pmid:33024307");
        assert_eq!(
            citations[0].url,
            Some("https://doi.org/10.1038/s41579-020-00459-7".to_string())
        );
    }

    #[test]
    fn test_citation_with_url_span() {
        // Given: Markdown with a citation and URL
        let markdown = "Text [@item-1](https://example.com) more text.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: The span includes the entire citation with URL
        assert_eq!(citations.len(), 1);
        let (start, end) = citations[0].span;
        assert_eq!(&markdown[start..end], "[@item-1](https://example.com)");
    }

    #[test]
    fn test_citation_without_url_still_works() {
        // Given: Markdown with a citation without URL (regression test)
        let markdown = "See [@item-1] for details.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: Citation is found with no URL
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[0].url, None);
    }

    #[test]
    fn test_mixed_citations_with_and_without_urls() {
        // Given: Markdown with some citations having URLs and some not
        let markdown = "First [@item-1](https://doi.org/1) and second [@item-2] citations.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: Both citations are found with correct URL status
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[0].url, Some("https://doi.org/1".to_string()));
        assert_eq!(citations[1].id, "item-2");
        assert_eq!(citations[1].url, None);
    }

    #[test]
    fn test_citation_url_with_special_chars() {
        // Given: URL with special characters (query params, fragments)
        let markdown = "See [@item-1](https://example.com/path?query=1&foo=bar#section) here.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: URL is captured correctly including special chars
        assert_eq!(citations.len(), 1);
        assert_eq!(
            citations[0].url,
            Some("https://example.com/path?query=1&foo=bar#section".to_string())
        );
    }

    // Sub-phase 2.3: Locator parsing tests

    #[test]
    fn test_citation_with_page_locator() {
        // Given: Markdown with a citation containing a page locator
        let markdown = "Voir [@book-1, p. 42] pour plus de détails.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with locator and label
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "book-1");
        assert_eq!(citations[0].locator, Some("42".to_string()));
        assert_eq!(citations[0].label, Some("page".to_string()));
    }

    #[test]
    fn test_citation_with_page_range() {
        // Given: Markdown with a citation containing a page range
        let markdown = "Cette section [@article-1, pp. 10-20] décrit la méthodologie.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with page range
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "article-1");
        assert_eq!(citations[0].locator, Some("10-20".to_string()));
        assert_eq!(citations[0].label, Some("page".to_string()));
    }

    #[test]
    fn test_citation_with_chapter_locator() {
        // Given: Markdown with a citation containing a chapter locator
        let markdown = "See [@book-2, ch. 3] for the introduction.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with chapter locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "book-2");
        assert_eq!(citations[0].locator, Some("3".to_string()));
        assert_eq!(citations[0].label, Some("chapter".to_string()));
    }

    #[test]
    fn test_citation_with_section_locator() {
        // Given: Markdown with a citation containing a section locator
        let markdown = "Refer to [@manual-1, sec. 4.2] for details.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with section locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "manual-1");
        assert_eq!(citations[0].locator, Some("4.2".to_string()));
        assert_eq!(citations[0].label, Some("section".to_string()));
    }

    #[test]
    fn test_citation_with_full_page_label() {
        // Given: Markdown with "page" instead of "p."
        let markdown = "Check [@doc-1, page 15] for the formula.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with page locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "doc-1");
        assert_eq!(citations[0].locator, Some("15".to_string()));
        assert_eq!(citations[0].label, Some("page".to_string()));
    }

    #[test]
    fn test_citation_with_full_pages_label() {
        // Given: Markdown with "pages" instead of "pp."
        let markdown = "See [@doc-2, pages 5-10] for examples.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with pages locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "doc-2");
        assert_eq!(citations[0].locator, Some("5-10".to_string()));
        assert_eq!(citations[0].label, Some("page".to_string()));
    }

    #[test]
    fn test_citation_with_full_chapter_label() {
        // Given: Markdown with "chapter" instead of "ch."
        let markdown = "Read [@book-3, chapter 7] first.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with chapter locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "book-3");
        assert_eq!(citations[0].locator, Some("7".to_string()));
        assert_eq!(citations[0].label, Some("chapter".to_string()));
    }

    #[test]
    fn test_citation_with_full_section_label() {
        // Given: Markdown with "section" instead of "sec."
        let markdown = "Consult [@guide-1, section 2.1] for setup.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find the citation with section locator
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].id, "guide-1");
        assert_eq!(citations[0].locator, Some("2.1".to_string()));
        assert_eq!(citations[0].label, Some("section".to_string()));
    }

    #[test]
    fn test_citation_locator_span() {
        // Given: Markdown with a locator citation
        let markdown = "Text [@item-1, p. 42] more text.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: The span correctly includes the entire citation with locator
        assert_eq!(citations.len(), 1);
        let (start, end) = citations[0].span;
        assert_eq!(&markdown[start..end], "[@item-1, p. 42]");
    }

    #[test]
    fn test_mixed_citations_with_and_without_locators() {
        // Given: Markdown with both types of citations
        let markdown = "First [@item-1] and second [@item-2, p. 10] in text.";

        // When: We extract citations
        let citations = extract_citations(markdown);

        // Then: We find both citations with correct properties
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].id, "item-1");
        assert_eq!(citations[0].locator, None);
        assert_eq!(citations[0].label, None);
        assert_eq!(citations[1].id, "item-2");
        assert_eq!(citations[1].locator, Some("10".to_string()));
        assert_eq!(citations[1].label, Some("page".to_string()));
    }
}
