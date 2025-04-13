//! Cryptographic provider factory
//!
//! This module provides factory functions for creating cryptographic providers.
//! It handles provider selection based on availability and configuration.

use std::env;
use std::path::{Path, PathBuf};
use std::sync::Once;
use once_cell::sync::OnceCell;

use crate::common::{Result, ProxyError};
use super::{CryptoProvider, ProviderType, StandardProvider, OqsProvider};

// Static initialization
static OQS_CHECK: Once = Once::new();
static OQS_AVAILABLE: OnceCell<bool> = OnceCell::new();
static OQS_PATH: OnceCell<Option<PathBuf>> = OnceCell::new();

// Provider singletons for better performance
static STANDARD_PROVIDER: OnceCell<StandardProvider> = OnceCell::new();
static OQS_PROVIDER: OnceCell<OqsProvider> = OnceCell::new();

// Logging flags to avoid duplicate logs
static LOGGED_OQS_WARNING: OnceCell<bool> = OnceCell::new();
static LOGGED_OQS_INFO: OnceCell<bool> = OnceCell::new();

/// Get the standard provider singleton
///
/// This function returns a reference to the standard provider singleton.
/// The provider is initialized on first access.
fn get_standard_provider() -> &'static StandardProvider {
    STANDARD_PROVIDER.get_or_init(|| {
        log::trace!("Initializing standard OpenSSL provider");
        StandardProvider::new()
    })
}

/// Get the OQS provider singleton
///
/// This function returns a reference to the OQS provider singleton if available.
/// The provider is initialized on first access.
fn get_oqs_provider() -> Option<&'static OqsProvider> {
    // Check if OQS is available
    if !is_oqs_available() {
        return None;
    }

    // Initialize the provider if needed
    if OQS_PROVIDER.get().is_none() {
        let provider = OqsProvider::new();
        OQS_PROVIDER.set(provider).ok();
    }

    OQS_PROVIDER.get()
}

/// Create a cryptographic provider based on the specified type
///
/// # Arguments
///
/// * `provider_type` - The type of provider to create (Standard, OQS, or Auto)
///
/// # Returns
///
/// A boxed provider implementing the CryptoProvider trait
pub fn create_provider(provider_type: ProviderType) -> Result<Box<dyn CryptoProvider>> {
    // Initialize OQS check if not already done
    initialize_oqs_check();

    match provider_type {
        ProviderType::Standard => {
            // Return the standard provider
            let provider = get_standard_provider();
            // Clone the provider to avoid lifetime issues
            Ok(Box::new(provider.clone()))
        },
        ProviderType::Oqs => {
            // Return the OQS provider if available
            if let Some(provider) = get_oqs_provider() {
                log::debug!("Using OQS-OpenSSL provider");
                // Clone the provider to avoid lifetime issues
                Ok(Box::new(provider.clone()))
            } else {
                Err(ProxyError::Certificate("OQS-OpenSSL provider not available. Install OQS-OpenSSL or use the standard provider.".to_string()))
            }
        },
        ProviderType::Auto => {
            // Try OQS provider first, fall back to standard if not available
            if let Some(provider) = get_oqs_provider() {
                // Log OQS availability (only once)
                if LOGGED_OQS_INFO.get().is_none() {
                    log::info!("Using OQS-OpenSSL provider with post-quantum support");
                    LOGGED_OQS_INFO.set(true).ok();
                }

                // Clone the provider to avoid lifetime issues
                Ok(Box::new(provider.clone()))
            } else {
                // Log fallback warning (only once)
                if LOGGED_OQS_WARNING.get().is_none() {
                    log::warn!("OQS-OpenSSL not available, falling back to standard OpenSSL (no post-quantum support)");
                    LOGGED_OQS_WARNING.set(true).ok();
                }

                let provider = get_standard_provider();
                // Clone the provider to avoid lifetime issues
                Ok(Box::new(provider.clone()))
            }
        }
    }
}

/// Check if OQS-OpenSSL is available
///
/// This function checks if OQS-OpenSSL is available on the system.
/// It caches the result for subsequent calls.
///
/// # Returns
///
/// `true` if OQS-OpenSSL is available, `false` otherwise
pub fn is_oqs_available() -> bool {
    // Initialize if not already initialized
    initialize_oqs_check();

    // Return cached result
    *OQS_AVAILABLE.get_or_init(|| false)
}

/// Get the path to OQS-OpenSSL installation
///
/// # Returns
///
/// Some(PathBuf) if OQS-OpenSSL is found, None otherwise
pub fn get_oqs_path() -> Option<PathBuf> {
    // Initialize if not already initialized
    initialize_oqs_check();

    // Return cached result
    OQS_PATH.get().cloned().unwrap_or(None)
}

/// Initialize OQS check
///
/// This function initializes the OQS check if not already initialized.
fn initialize_oqs_check() {
    // Initialize once
    OQS_CHECK.call_once(|| {
        // Try to find OQS-OpenSSL
        let oqs_path = find_oqs_openssl();
        let available = oqs_path.is_some();

        // Store the results
        OQS_AVAILABLE.set(available).ok();
        OQS_PATH.set(oqs_path.clone()).ok();

        // Log the result (only once during initialization)
        if available {
            log::info!("OQS-OpenSSL detected at {:?}", oqs_path.unwrap());
        } else {
            log::debug!("OQS-OpenSSL not detected");
        }
    });
}

/// Find OQS-OpenSSL installation
///
/// This function tries to find OQS-OpenSSL installation by checking:
/// 1. Environment variables
/// 2. Common installation paths
/// 3. Dynamic library loading
///
/// # Returns
///
/// Some(PathBuf) if OQS-OpenSSL is found, None otherwise
fn find_oqs_openssl() -> Option<PathBuf> {
    // Check environment variables
    if let Ok(path) = env::var("OQS_OPENSSL_PATH") {
        let path = PathBuf::from(path);
        log::debug!("Checking OQS_OPENSSL_PATH: {}", path.display());
        if path.exists() {
            log::debug!("Path exists: {}", path.display());
            if is_valid_oqs_path(&path) {
                log::debug!("Valid OQS path found: {}", path.display());
                return Some(path);
            } else {
                log::debug!("Path exists but is not a valid OQS path: {}", path.display());
            }
        } else {
            log::debug!("Path does not exist: {}", path.display());
        }
    } else {
        log::debug!("OQS_OPENSSL_PATH environment variable not set");
    }

    // Check common installation paths
    let common_paths = [
        "/opt/oqs-openssl",
        "/usr/local/opt/oqs-openssl",
        "/usr/local/oqs-openssl",
        "/usr/opt/oqs-openssl",
        // Add paths for OpenSSL 3.x with OQS provider
        "/opt/oqs/openssl",
        "/usr/local/opt/oqs/openssl",
        "/usr/local/oqs/openssl",
    ];

    for &path_str in &common_paths {
        let path = PathBuf::from(path_str);
        log::debug!("Checking common path: {}", path.display());
        if path.exists() {
            log::debug!("Path exists: {}", path.display());
            if is_valid_oqs_path(&path) {
                log::debug!("Valid OQS path found: {}", path.display());
                return Some(path);
            } else {
                log::debug!("Path exists but is not a valid OQS path: {}", path.display());
            }
        }
    }

    // Try to dynamically load OQS library
    // This is a simplified check; a real implementation would try to load the library

    None
}

/// Check if a path is a valid OQS-OpenSSL installation
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if the path contains a valid OQS-OpenSSL installation, `false` otherwise
fn is_valid_oqs_path(path: &Path) -> bool {
    // First, check if this is a standard OQS-OpenSSL installation
    // with bin/openssl and lib/liboqs.so
    let standard_check = || {
        // Check for lib directory
        let lib_path = path.join("lib");
        if !lib_path.exists() || !lib_path.is_dir() {
            return false;
        }

        // Check for bin directory
        let bin_path = path.join("bin");
        if !bin_path.exists() || !bin_path.is_dir() {
            return false;
        }

        // Check for openssl executable
        let openssl_path = bin_path.join("openssl");
        if !openssl_path.exists() {
            return false;
        }

        // Check for liboqs library
        let liboqs_path = lib_path.join("liboqs.so");
        let liboqs_path_alt = lib_path.join("liboqs.dylib");
        if !liboqs_path.exists() && !liboqs_path_alt.exists() {
            return false;
        }

        true
    };

    // If standard check passes, return true
    if standard_check() {
        return true;
    }

    // If standard check fails, check for OpenSSL 3.x with OQS provider
    // This is for installations where OpenSSL and liboqs are in separate directories

    // Check for bin directory with openssl executable
    let bin_path = path.join("bin");
    let openssl_path = bin_path.join("openssl");
    if !bin_path.exists() || !bin_path.is_dir() || !openssl_path.exists() {
        return false;
    }

    // Check for lib64/ossl-modules directory with oqsprovider.so
    let modules_path = path.join("lib64").join("ossl-modules");
    let oqsprovider_path = modules_path.join("oqsprovider.so");
    if !modules_path.exists() || !modules_path.is_dir() || !oqsprovider_path.exists() {
        return false;
    }

    // If we get here, we have a valid OpenSSL 3.x with OQS provider
    true
}
