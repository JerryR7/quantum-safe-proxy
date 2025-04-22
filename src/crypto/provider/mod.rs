//! Cryptographic provider module
//!
//! This module provides cryptographic providers for TLS operations.
//! It abstracts the underlying cryptographic library (OpenSSL) and
//! provides a unified interface for TLS operations.

mod factory;
#[cfg(feature = "openssl")]
mod openssl;
mod capabilities;
pub mod environment;
mod fallback;

// Re-exports
pub use factory::create_provider;
pub use environment::{check_environment, diagnose_environment, EnvironmentInfo, EnvironmentIssue, IssueSeverity};
pub use capabilities::get_openssl_version;
pub use capabilities::get_supported_pq_algorithms;
pub use capabilities::get_supported_signature_algorithms;
pub use capabilities::is_openssl35_available;
pub use capabilities::is_pqc_available;
pub use capabilities::get_recommended_cipher_list;
pub use capabilities::get_recommended_groups;
pub use factory::is_oqs_available;

// Import OpenSSL types
use std::path::Path;
use crate::common::Result;

// Define our own types to avoid direct dependency on OpenSSL types
// This allows for better flexibility with different OpenSSL versions

// Type aliases for OpenSSL types
#[cfg(feature = "openssl")]
pub use ::openssl::ssl::SslContext;
#[cfg(feature = "openssl")]
pub use ::openssl::x509::X509;

// Fallback definitions when OpenSSL is not available
#[cfg(not(feature = "openssl"))]
pub struct SslContext;
#[cfg(not(feature = "openssl"))]
pub struct X509;

/// Provider type
///
/// This enum represents the type of cryptographic provider to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// Standard OpenSSL provider
    Standard,

    /// OQS-OpenSSL provider (for backward compatibility)
    Oqs,

    /// Automatically select the best available provider
    Auto,
}

/// Certificate type
///
/// This enum represents the type of certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateType {
    /// Traditional certificate (RSA, ECDSA, etc.)
    Traditional,

    /// Hybrid certificate (traditional + post-quantum)
    Hybrid,

    /// Pure post-quantum certificate
    PostQuantum,
}

/// Cryptographic capabilities
///
/// This structure holds the capabilities of a cryptographic provider.
#[derive(Debug, Clone)]
pub struct CryptoCapabilities {
    /// Whether the provider supports post-quantum cryptography
    pub supports_pqc: bool,

    /// Supported key exchange algorithms
    pub supported_key_exchange: Vec<String>,

    /// Supported signature algorithms
    pub supported_signatures: Vec<String>,

    /// Recommended TLS cipher list
    pub recommended_cipher_list: String,

    /// Recommended TLS 1.3 ciphersuites
    pub recommended_tls13_ciphersuites: String,

    /// Recommended TLS groups
    pub recommended_groups: String,
}

/// Cryptographic provider trait
///
/// This trait defines the interface for cryptographic providers.
pub trait CryptoProvider: Send + Sync + std::fmt::Debug {
    /// Create a TLS server context
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the server certificate
    /// * `key_path` - Path to the server private key
    /// * `ca_path` - Optional path to the CA certificate
    ///
    /// # Returns
    ///
    /// A TLS server context
    fn create_server_context(&self, cert_path: &Path, key_path: &Path, ca_path: Option<&Path>) -> Result<SslContext>;

    /// Create a TLS client context
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Optional path to the client certificate
    /// * `key_path` - Optional path to the client private key
    /// * `ca_path` - Optional path to the CA certificate
    ///
    /// # Returns
    ///
    /// A TLS client context
    fn create_client_context(&self, cert_path: Option<&Path>, key_path: Option<&Path>, ca_path: Option<&Path>) -> Result<SslContext>;

    /// Get the provider's capabilities
    ///
    /// # Returns
    ///
    /// The provider's capabilities
    fn capabilities(&self) -> CryptoCapabilities;

    /// Get the provider's name
    ///
    /// # Returns
    ///
    /// The provider's name
    fn name(&self) -> &'static str;

    /// Check if a certificate is a hybrid certificate
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// `true` if the certificate is a hybrid certificate, `false` otherwise
    fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool>;

    /// Get certificate subject information
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The certificate subject information as a string
    fn get_cert_subject(&self, cert_path: &Path) -> Result<String>;

    /// Get certificate fingerprint
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The certificate fingerprint as a string
    fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String>;

    /// Load certificate from PEM file
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The loaded X509 certificate
    fn load_cert(&self, cert_path: &Path) -> Result<X509>;
}
