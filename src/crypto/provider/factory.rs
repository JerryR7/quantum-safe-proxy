//! Cryptographic provider factory
//!
//! This module provides factory functions for creating cryptographic providers.
//! It handles provider selection based on availability and configuration.

use std::env;
use std::path::{Path, PathBuf};
use std::sync::Once;

use crate::common::{Result, ProxyError};
use super::{CryptoProvider, ProviderType, StandardProvider, OqsProvider};

// Static initialization
static OQS_CHECK: Once = Once::new();
static mut OQS_AVAILABLE: bool = false;
static mut OQS_PATH: Option<PathBuf> = None;

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
    match provider_type {
        ProviderType::Standard => {
            log::info!("Using standard OpenSSL provider (no post-quantum support)");
            Ok(Box::new(StandardProvider::new()))
        },
        ProviderType::Oqs => {
            if is_oqs_available() {
                log::info!("Using OQS-OpenSSL provider with post-quantum support");
                Ok(Box::new(OqsProvider::new()))
            } else {
                Err(ProxyError::Certificate("OQS-OpenSSL provider not available. Install OQS-OpenSSL or use the standard provider.".to_string()))
            }
        },
        ProviderType::Auto => {
            if is_oqs_available() {
                log::info!("Automatically selected OQS-OpenSSL provider with post-quantum support");
                Ok(Box::new(OqsProvider::new()))
            } else {
                log::warn!("OQS-OpenSSL not available, falling back to standard OpenSSL (no post-quantum support)");
                Ok(Box::new(StandardProvider::new()))
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
    // Initialize once
    OQS_CHECK.call_once(|| {
        unsafe {
            // Try to find OQS-OpenSSL
            OQS_AVAILABLE = find_oqs_openssl().is_some();
            
            // Log the result
            if OQS_AVAILABLE {
                log::info!("OQS-OpenSSL detected at {:?}", OQS_PATH.as_ref().unwrap());
            } else {
                log::warn!("OQS-OpenSSL not detected");
            }
        }
    });
    
    // Return cached result
    unsafe { OQS_AVAILABLE }
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
        if path.exists() && is_valid_oqs_path(&path) {
            unsafe { OQS_PATH = Some(path.clone()); }
            return Some(path);
        }
    }
    
    // Check common installation paths
    let common_paths = [
        "/opt/oqs-openssl",
        "/usr/local/opt/oqs-openssl",
        "/usr/local/oqs-openssl",
        "/usr/opt/oqs-openssl",
    ];
    
    for &path_str in &common_paths {
        let path = PathBuf::from(path_str);
        if path.exists() && is_valid_oqs_path(&path) {
            unsafe { OQS_PATH = Some(path.clone()); }
            return Some(path);
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
}
