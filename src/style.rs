//! CSL style loading.
//!
//! Handles loading CSL style files and provides access to built-in styles.

use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading styles.
#[derive(Error, Debug)]
pub enum StyleError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid CSL style: {0}")]
    InvalidStyle(String),
}

/// Loads a CSL style from a file.
///
/// # Arguments
///
/// * `path` - Path to the .csl file
///
/// # Returns
///
/// The CSL XML content as a string.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn load_style(path: &Path) -> Result<String, StyleError> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

/// Returns a built-in style by name.
///
/// # Arguments
///
/// * `name` - The name of the built-in style (e.g., "apa", "ieee")
///
/// # Returns
///
/// The CSL XML content if the style exists, or None.
pub fn builtin_style(name: &str) -> Option<&'static str> {
    match name {
        "minimal" => Some(MINIMAL_STYLE),
        _ => None,
    }
}

/// Minimal CSL style for testing purposes.
const MINIMAL_STYLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info>
    <title>Minimal Style</title>
    <id>minimal</id>
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
      <text variable="title"/>
    </layout>
  </bibliography>
</style>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_styles_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/styles")
    }

    // ============================================
    // Tests for load_style()
    // ============================================

    #[test]
    fn test_load_style_valid_csl_file() {
        // Given: A valid CSL file exists
        let path = test_styles_dir().join("minimal.csl");

        // When: We load the style
        let result = load_style(&path);

        // Then: We get the XML content
        assert!(result.is_ok(), "Should load valid CSL file");
        let content = result.unwrap();
        assert!(
            content.contains("<style"),
            "Should contain CSL style element"
        );
        assert!(
            content.contains("Minimal Test Style"),
            "Should contain style title"
        );
    }

    #[test]
    fn test_load_style_missing_file() {
        // Given: A non-existent file path
        let path = test_styles_dir().join("nonexistent.csl");

        // When: We try to load the style
        let result = load_style(&path);

        // Then: We get an IO error
        assert!(result.is_err(), "Should fail for missing file");
        let err = result.unwrap_err();
        assert!(
            matches!(err, StyleError::IoError(_)),
            "Should be an IoError, got: {:?}",
            err
        );
    }

    #[test]
    fn test_load_style_returns_complete_content() {
        // Given: A valid CSL file
        let path = test_styles_dir().join("minimal.csl");

        // When: We load the style
        let content = load_style(&path).unwrap();

        // Then: The content includes all expected CSL elements
        assert!(content.contains("<?xml"), "Should contain XML declaration");
        assert!(
            content.contains("<citation>"),
            "Should contain citation element"
        );
        assert!(
            content.contains("<bibliography>"),
            "Should contain bibliography element"
        );
        assert!(
            content.contains("</style>"),
            "Should contain closing style tag"
        );
    }

    // ============================================
    // Tests for builtin_style()
    // ============================================

    #[test]
    fn test_builtin_style_unknown_returns_none() {
        // Given: An unknown style name
        let name = "unknown-style-that-does-not-exist";

        // When: We request the builtin style
        let result = builtin_style(name);

        // Then: We get None
        assert!(result.is_none(), "Unknown style should return None");
    }

    #[test]
    fn test_builtin_style_minimal() {
        // Given: The "minimal" builtin style name
        let name = "minimal";

        // When: We request the builtin style
        let result = builtin_style(name);

        // Then: We get a valid CSL style
        assert!(result.is_some(), "minimal style should be available");
        let content = result.unwrap();
        assert!(
            content.contains("<style"),
            "Should contain CSL style element"
        );
        assert!(
            content.contains("<citation>"),
            "Should contain citation element"
        );
    }
}
