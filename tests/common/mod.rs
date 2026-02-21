//! Shared test constants and helpers for integration tests.

/// Numeric CSL style with `collapse="citation-number"` and NO `<sort>` in `<bibliography>`.
///
/// Because there is no sort key in the bibliography section, csl_proc preserves
/// the order of the refs array we pass in. This is the style to use when testing
/// that bibliography ordering matches citation-appearance order.
///
/// Also used in `src/processor.rs` unit tests as `NUMERIC_NOSORT_STYLE`
/// (duplicated there because unit tests cannot import from integration test crates).
pub const NUMERIC_STYLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <id>numeric-collapse</id>
    <title>Numeric with Collapse</title>
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

/// Build a JSON array of test references from a list of IDs.
///
/// Each reference gets an auto-generated author (`AuthorX` where X is the last
/// char of the ID) and a title (`Title {id}`), all dated 2020.
pub fn build_refs(ids: &[&str]) -> String {
    let refs: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                r#"{{"id": "{}", "type": "article-journal", "author": [{{"family": "Author{}", "given": "A."}}], "title": "Title {}", "issued": {{"date-parts": [[2020]]}}}}"#,
                id,
                id.chars().last().unwrap_or('X'),
                id
            )
        })
        .collect();
    format!("[{}]", refs.join(", "))
}
