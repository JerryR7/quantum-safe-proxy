//! Cryptographic provider factory
//!
//! This module provides factory functions for creating cryptographic providers.
//! It handles provider selection based on availability and configuration.

use std::sync::Once;
use once_cell::sync::OnceCell;

use crate::common::Result;
use super::{CryptoProvider, ProviderType};
use super::api;

// Provider singletons for better performance
#[cfg(feature = "openssl")]
static OPENSSL_PROVIDER: OnceCell<super::openssl::OpenSSLProvider> = OnceCell::new();

#[cfg(not(feature = "openssl"))]
static FALLBACK_PROVIDER: OnceCell<super::fallback::FallbackProvider> = OnceCell::new();

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
            #[cfg(feature = "openssl")]
            {
                // Get the OpenSSL provider
                let provider = get_openssl_provider();
                return Ok(Box::new(provider.clone()));
            }

            #[cfg(not(feature = "openssl"))]
            {
                // Get the fallback provider
                let provider = get_fallback_provider();
                return Ok(Box::new(provider.clone()));
            }
        }
    }
}

#[cfg(feature = "openssl")]
/// Get the OpenSSL provider singleton
///
/// This function returns a reference to the OpenSSL provider singleton.
/// The provider is initialized on first access.
fn get_openssl_provider() -> &'static super::openssl::OpenSSLProvider {
    // Initialize provider if needed
    initialize_provider();

    // Return the provider
    OPENSSL_PROVIDER.get().expect("OpenSSL provider not initialized")
}

#[cfg(not(feature = "openssl"))]
/// Get the fallback provider singleton
///
/// This function returns a reference to the fallback provider singleton.
/// The provider is initialized on first access.
fn get_fallback_provider() -> &'static super::fallback::FallbackProvider {
    // Initialize provider if needed
    initialize_provider();

    // Return the provider
    FALLBACK_PROVIDER.get().expect("Fallback provider not initialized")
}

/// Initialize the providers
///
/// This function initializes the provider singletons if not already initialized.
fn initialize_provider() {
    INIT_CHECK.call_once(|| {
        #[cfg(feature = "openssl")]
        {
            // Create the OpenSSL provider
            let provider = super::openssl::OpenSSLProvider::new();

            // Log provider information
            let capabilities = provider.capabilities();
            if capabilities.supports_pqc {
                log::info!("Using {} with post-quantum support", provider.name());
            } else {
                log::warn!("Using {} without post-quantum support", provider.name());
            }

            // Store the provider
            OPENSSL_PROVIDER.set(provider).ok();
        }

        #[cfg(not(feature = "openssl"))]
        {
            // Create the fallback provider
            let provider = super::fallback::FallbackProvider {};
            log::warn!("Using fallback provider: {}", provider.name());

            // Store the provider
            FALLBACK_PROVIDER.set(provider).ok();
        }
    });
}

/// Check if post-quantum cryptography is available
///
/// This function checks if post-quantum cryptography is available.
/// It directly checks the provider's capabilities to determine if PQC is supported,
/// regardless of the specific implementation.
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
