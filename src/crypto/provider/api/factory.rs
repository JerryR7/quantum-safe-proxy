//! API strategy factory
//!
//! This module provides a factory function for creating API strategies.

use std::sync::Arc;
use once_cell::sync::OnceCell;
use log::debug;

use super::{OpenSSLApiStrategy, OpenSSLApiImpl};
#[cfg(not(feature = "openssl"))]
use super::CommandLineImpl;

// API strategy singleton
static API_STRATEGY: OnceCell<Arc<dyn OpenSSLApiStrategy>> = OnceCell::new();

/// Get the API strategy
///
/// This function returns a reference to the API strategy singleton.
/// The strategy is initialized on first access.
///
/// # Returns
///
/// A reference to the API strategy
pub fn get_api_strategy() -> &'static Arc<dyn OpenSSLApiStrategy> {
    API_STRATEGY.get_or_init(|| {
        debug!("Initializing OpenSSL API strategy");

        // Try to use the OpenSSL API
        #[cfg(feature = "openssl")]
        {
            debug!("Using OpenSSL API strategy");
            Arc::new(OpenSSLApiImpl) as Arc<dyn OpenSSLApiStrategy>
        }
        #[cfg(not(feature = "openssl"))]
        {
            // If OpenSSL API is not available, use command-line
            debug!("Using command-line strategy (OpenSSL API not available)");
            Arc::new(CommandLineImpl) as Arc<dyn OpenSSLApiStrategy>
        }
    })
}
