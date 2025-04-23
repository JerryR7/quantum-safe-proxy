//! Filesystem utility functions
//!
//! This module provides utility functions for filesystem operations.

use std::path::Path;
use std::fs;

use super::error::{ProxyError, Result};

/// Check if a file exists
///
/// # Arguments
///
/// * `path` - File path
///
/// # Returns
///
/// Returns `Ok(())` if the file exists, otherwise returns an error.
pub fn check_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ProxyError::Config(format!(
            "File does not exist: {:?}",
            path
        )));
    }

    if !path.is_file() {
        return Err(ProxyError::Config(format!(
            "Path is not a file: {:?}",
            path
        )));
    }

    Ok(())
}

/// Read file content
///
/// # Arguments
///
/// * `path` - File path
///
/// # Returns
///
/// Returns the file content as a byte vector.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    check_file_exists(path)?;

    fs::read(path).map_err(|e| ProxyError::Io(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_check_file_exists() {
        // Test existing file
        let path = PathBuf::from("Cargo.toml");
        let result = check_file_exists(&path);
        assert!(result.is_ok(), "Should be able to check an existing file");

        // Test non-existent file
        let path = PathBuf::from("non_existent_file.txt");
        let result = check_file_exists(&path);
        assert!(result.is_err(), "Should fail when checking a non-existent file");
    }

    #[test]
    fn test_read_file() {
        // Test reading an existing file
        let path = PathBuf::from("Cargo.toml");
        let result = read_file(&path);
        assert!(result.is_ok(), "Should be able to read an existing file");

        if let Ok(content) = result {
            assert!(!content.is_empty(), "File content should not be empty");
        }

        // Test reading a non-existent file
        let path = PathBuf::from("non_existent_file.txt");
        let result = read_file(&path);
        assert!(result.is_err(), "Should fail when reading a non-existent file");
    }
}
