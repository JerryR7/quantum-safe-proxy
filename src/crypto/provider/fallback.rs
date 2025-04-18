//! Fallback cryptographic provider
//!
//! This module provides a fallback implementation of the CryptoProvider trait
//! for when OpenSSL is not available. It returns appropriate errors for all operations.

use std::path::Path;
use std::fmt;

use crate::common::{ProxyError, Result};
use super::{CryptoProvider, CryptoCapabilities, CertificateType, SslContext, X509};

/// Fallback cryptographic provider
///
/// This provider is used when OpenSSL is not available.
/// All operations return appropriate errors.
#[derive(Clone)]
pub struct FallbackProvider;

impl FallbackProvider {
    /// Create a new fallback provider
    pub fn new() -> Self {
        Self {}
    }
}

impl fmt::Debug for FallbackProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackProvider").finish()
    }
}

impl CryptoProvider for FallbackProvider {
    fn create_server_context(&self, _cert_path: &Path, _key_path: &Path, _ca_path: Option<&Path>) -> Result<SslContext> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }

    fn create_client_context(&self, _cert_path: Option<&Path>, _key_path: Option<&Path>, _ca_path: Option<&Path>) -> Result<SslContext> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }

    fn capabilities(&self) -> CryptoCapabilities {
        CryptoCapabilities {
            supports_pqc: false,
            supported_key_exchange: Vec::new(),
            supported_signatures: Vec::new(),
            recommended_cipher_list: String::new(),
            recommended_tls13_ciphersuites: String::new(),
            recommended_groups: String::new(),
        }
    }

    fn name(&self) -> &'static str {
        "Fallback (OpenSSL not available)"
    }

    fn is_hybrid_cert(&self, _cert_path: &Path) -> Result<bool> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }

    fn get_cert_subject(&self, _cert_path: &Path) -> Result<String> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }

    fn get_cert_fingerprint(&self, _cert_path: &Path) -> Result<String> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }

    fn load_cert(&self, _cert_path: &Path) -> Result<X509> {
        Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()))
    }
}
