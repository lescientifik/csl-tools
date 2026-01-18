//! CSL-JSON reference loading.
//!
//! Handles loading references from JSON files, supporting both
//! standard JSON arrays and JSONL format (one JSON object per line).

use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading references.
#[derive(Error, Debug)]
pub enum RefsError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid JSONL at line {line}: {message}")]
    JsonlError { line: usize, message: String },

    #[error("References must be a JSON array")]
    NotAnArray,
}

/// Loads references from a CSL-JSON or JSONL file.
///
/// # Arguments
///
/// * `path` - Path to the references file
///
/// # Returns
///
/// A JSON string containing an array of references.
///
/// # Errors
///
/// Returns an error if the file cannot be read or contains invalid JSON.
pub fn load_refs(path: &Path) -> Result<String, RefsError> {
    let content = fs::read_to_string(path)?;
    normalize_refs(&content)
}

/// Validates that the given JSON string contains valid CSL-JSON references.
pub fn validate_refs(json: &str) -> Result<(), RefsError> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    if !value.is_array() {
        return Err(RefsError::NotAnArray);
    }
    Ok(())
}

/// Normalizes reference content to a JSON array.
///
/// Supports two input formats:
/// - JSON array: `[{"id": "1", ...}, {"id": "2", ...}]`
/// - JSONL: `{"id": "1", ...}\n{"id": "2", ...}`
///
/// # Returns
///
/// A JSON string containing an array of references.
fn normalize_refs(content: &str) -> Result<String, RefsError> {
    let trimmed = content.trim();

    // Empty content returns empty array
    if trimmed.is_empty() {
        return Ok("[]".to_string());
    }

    // Check if it's already a JSON array
    if trimmed.starts_with('[') {
        // Validate it's valid JSON
        let value: serde_json::Value = serde_json::from_str(trimmed)?;
        if !value.is_array() {
            return Err(RefsError::NotAnArray);
        }
        // Return the normalized (re-serialized) JSON
        return Ok(serde_json::to_string(&value)?);
    }

    // Treat as JSONL: parse each non-empty line as a JSON object
    let mut refs: Vec<serde_json::Value> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => refs.push(value),
            Err(e) => {
                return Err(RefsError::JsonlError {
                    line: line_num + 1, // 1-indexed line numbers
                    message: e.to_string(),
                });
            }
        }
    }

    Ok(serde_json::to_string(&refs)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to create a temporary file with content
    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    // --- Tests for load_refs ---

    #[test]
    fn test_load_refs_json_array() {
        // Given: a file containing a valid JSON array of references
        let content = r#"[{"id": "item-1", "type": "book", "title": "Test Book"}]"#;
        let file = create_temp_file(content);

        // When: we load the references
        let result = load_refs(file.path());

        // Then: we get the JSON array string back
        assert!(result.is_ok());
        let json = result.unwrap();
        // Verify it parses as a JSON array
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_load_refs_jsonl() {
        // Given: a file containing JSONL format (one JSON object per line)
        let content = r#"{"id": "item-1", "type": "book", "title": "Book One"}
{"id": "item-2", "type": "article", "title": "Article Two"}"#;
        let file = create_temp_file(content);

        // When: we load the references
        let result = load_refs(file.path());

        // Then: we get a JSON array string with both items
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_load_refs_file_not_found() {
        // Given: a path to a non-existent file
        let path = Path::new("/nonexistent/path/refs.json");

        // When: we try to load the references
        let result = load_refs(path);

        // Then: we get an IO error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RefsError::IoError(_)));
    }

    #[test]
    fn test_load_refs_invalid_json() {
        // Given: a file with invalid JSON
        let content = r#"{"id": "item-1", invalid json"#;
        let file = create_temp_file(content);

        // When: we try to load the references
        let result = load_refs(file.path());

        // Then: we get a JSON error
        assert!(result.is_err());
    }

    #[test]
    fn test_load_refs_empty_file() {
        // Given: an empty file
        let file = create_temp_file("");

        // When: we load the references
        let result = load_refs(file.path());

        // Then: we get an empty array
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    // --- Tests for normalize_refs ---

    #[test]
    fn test_normalize_refs_json_array() {
        // Given: a JSON array string
        let content = r#"[{"id": "item-1"}, {"id": "item-2"}]"#;

        // When: we normalize it
        let result = normalize_refs(content);

        // Then: we get the same array back (normalized)
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_normalize_refs_jsonl() {
        // Given: JSONL content
        let content = r#"{"id": "item-1"}
{"id": "item-2"}
{"id": "item-3"}"#;

        // When: we normalize it
        let result = normalize_refs(content);

        // Then: we get a JSON array
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_normalize_refs_jsonl_with_blank_lines() {
        // Given: JSONL with blank lines
        let content = r#"{"id": "item-1"}

{"id": "item-2"}"#;

        // When: we normalize it
        let result = normalize_refs(content);

        // Then: blank lines are ignored
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_normalize_refs_jsonl_error_with_line_number() {
        // Given: JSONL with invalid JSON on line 2
        let content = r#"{"id": "item-1"}
invalid json here
{"id": "item-3"}"#;

        // When: we try to normalize it
        let result = normalize_refs(content);

        // Then: we get an error indicating line 2
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            RefsError::JsonlError { line, .. } => assert_eq!(line, 2),
            _ => panic!("Expected JsonlError, got {:?}", err),
        }
    }

    #[test]
    fn test_normalize_refs_empty_string() {
        // Given: empty string
        let content = "";

        // When: we normalize it
        let result = normalize_refs(content);

        // Then: we get an empty array
        assert!(result.is_ok());
        let json = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    // --- Tests for validate_refs ---

    #[test]
    fn test_validate_refs_empty_array() {
        assert!(validate_refs("[]").is_ok());
    }

    #[test]
    fn test_validate_refs_valid_array() {
        let json = r#"[{"id": "item-1", "type": "book"}]"#;
        assert!(validate_refs(json).is_ok());
    }

    #[test]
    fn test_validate_refs_invalid_json() {
        let json = "not valid json";
        assert!(validate_refs(json).is_err());
    }

    #[test]
    fn test_validate_refs_not_array() {
        let json = r#"{"id": "item-1"}"#;
        // Should fail because refs must be an array
        assert!(validate_refs(json).is_err());
    }
}
