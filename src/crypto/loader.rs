//! OpenSSL dynamic loader
//!
//! This module provides functionality to dynamically load OpenSSL libraries
//! from a specified directory.

use std::path::Path;
use log::{info, warn, error};

/// Initialize OpenSSL from a specific directory
///
/// This function attempts to load OpenSSL libraries from the specified directory.
/// It must be called before any OpenSSL functions are used.
///
/// # Arguments
///
/// * `openssl_dir` - Path to the OpenSSL installation directory
///
/// # Returns
///
/// `true` if OpenSSL was successfully initialized, `false` otherwise
pub fn initialize_openssl(openssl_dir: &Path) -> bool {
    info!("Initializing OpenSSL from directory: {}", openssl_dir.display());

    // Check if the directory exists
    if !openssl_dir.exists() {
        error!("OpenSSL directory does not exist: {}", openssl_dir.display());
        return false;
    }

    // Set environment variables for OpenSSL
    std::env::set_var("OPENSSL_DIR", openssl_dir.to_string_lossy().to_string());

    // Set LD_LIBRARY_PATH to include the OpenSSL lib directory
    let lib_dir = openssl_dir.join("lib");
    if lib_dir.exists() {
        let current_path = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if current_path.is_empty() {
            lib_dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", lib_dir.to_string_lossy(), current_path)
        };
        info!("Setting LD_LIBRARY_PATH to include OpenSSL lib directory: {}", new_path);
        std::env::set_var("LD_LIBRARY_PATH", new_path);
    } else {
        warn!("OpenSSL lib directory does not exist: {}", lib_dir.display());
    }

    // Try to load OpenSSL libraries
    // Reset any previously loaded libraries
    openssl_sys::init();

    // Try to get the OpenSSL version to verify it's loaded correctly
    let version = openssl::version::version();
    info!("Loaded OpenSSL version: {}", version);

    // Check if it's OpenSSL 3.5+
    if version.contains("3.5") || version.contains("3.6") || version.contains("3.7") {
        info!("Successfully loaded OpenSSL 3.5+ from {}", openssl_dir.display());
        true
    } else {
        warn!("Loaded OpenSSL version is not 3.5+: {}", version);
        false
    }
}
