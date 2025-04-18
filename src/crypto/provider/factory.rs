//! Cryptographic provider factory
//!
//! This module provides factory functions for creating cryptographic providers.
//! It handles provider selection based on availability and configuration.

use std::sync::Once;
use once_cell::sync::OnceCell;

use crate::common::Result;
use super::{CryptoProvider, ProviderType};
use super::openssl::OpenSSLProvider;
use super::api;

// Provider singleton for better performance
static OPENSSL_PROVIDER: OnceCell<OpenSSLProvider> = OnceCell::new();

// Initialization flag
static INIT_CHECK: Once = Once::new();

/// Create a cryptographic provider based on the specified type
///
/// # Arguments
///
/// * `provider_type` - The type of provider to create
///
/// # Returns
///
/// A boxed provider implementing the CryptoProvider trait
pub fn create_provider(provider_type: ProviderType) -> Result<Box<dyn CryptoProvider>> {
    // Initialize provider if needed
    initialize_provider();

    match provider_type {
        ProviderType::Standard | ProviderType::Oqs | ProviderType::Auto => {
            // Get the OpenSSL provider
            let provider = get_openssl_provider();

            // Return the provider
            Ok(Box::new(provider.clone()))
        }
    }
}

/// Get the OpenSSL provider singleton
///
/// This function returns a reference to the OpenSSL provider singleton.
/// The provider is initialized on first access.
fn get_openssl_provider() -> &'static OpenSSLProvider {
    // Initialize provider if needed
    initialize_provider();

    // Return the provider
    OPENSSL_PROVIDER.get().expect("OpenSSL provider not initialized")
}

/// Initialize the provider
///
/// This function initializes the provider if not already initialized.
fn initialize_provider() {
    INIT_CHECK.call_once(|| {
        // Create the OpenSSL provider
        let provider = OpenSSLProvider::new();

        // Log provider information
        let capabilities = provider.capabilities();
        if capabilities.supports_pqc {
            log::info!("Using {} with post-quantum support", provider.name());
        } else {
            log::warn!("Using {} without post-quantum support", provider.name());
        }

        // Store the provider
        OPENSSL_PROVIDER.set(provider).ok();
    });
}

/// Check if post-quantum cryptography is available
///
/// This function checks if post-quantum cryptography is available
/// in the OpenSSL installation.
///
/// # Returns
///
/// `true` if post-quantum cryptography is available, `false` otherwise
pub fn is_pqc_available() -> bool {
    // Use the API layer's is_pqc_available function
    // This ensures consistent results across the application
    api::is_pqc_available()
}

/// Backward compatibility function for OQS availability
///
/// This function is provided for backward compatibility with code
/// that checks for OQS availability. It now checks for PQC support
/// in general, regardless of whether it's provided by OQS or OpenSSL 3.5.
///
/// # Returns
///
/// `true` if post-quantum cryptography is available, `false` otherwise
pub fn is_oqs_available() -> bool {
    is_pqc_available()
}

/// Check if OpenSSL 3.5+ is available
///
/// This function checks if OpenSSL 3.5+ is available in the system.
/// OpenSSL 3.5+ includes built-in post-quantum cryptography support.
///
/// # Returns
///
/// `true` if OpenSSL 3.5+ is available, `false` otherwise
#[allow(dead_code)]
pub fn is_openssl35_available() -> bool {
    api::is_openssl35_available()
}
