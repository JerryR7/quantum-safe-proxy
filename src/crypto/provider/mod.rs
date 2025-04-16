//! Cryptographic provider module
//!
//! This module defines the cryptographic provider trait and related types.
//! It abstracts different cryptographic implementations (standard OpenSSL and OQS-OpenSSL).

use std::path::Path;
use crate::common::Result;
use openssl::x509::X509;

// Forward declarations for submodules
mod standard;
mod oqs;
mod factory;
mod environment;

// Re-exports
pub use standard::StandardProvider;
pub use oqs::OqsProvider;
pub use factory::{create_provider, is_oqs_available};
pub use environment::{check_environment, diagnose_environment, EnvironmentInfo, EnvironmentIssue, IssueSeverity};

/// Cryptographic provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// Standard OpenSSL provider
    Standard,
    /// OQS-OpenSSL provider with post-quantum support
    Oqs,
    /// Automatically select the best available provider
    Auto,
}

impl Default for ProviderType {
    fn default() -> Self {
        Self::Auto
    }
}

/// Certificate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateType {
    /// Classical certificate (RSA, ECDSA, etc.)
    Classical,
    /// Hybrid certificate (classical + post-quantum)
    Hybrid,
    /// Pure post-quantum certificate
    PurePostQuantum,
    /// Unknown certificate type
    Unknown,
}

/// Cryptographic provider capabilities
#[derive(Debug, Clone)]
pub struct CryptoCapabilities {
    /// Whether post-quantum cryptography is supported
    pub supports_pqc: bool,
    /// Supported key exchange algorithms
    pub supported_key_exchange: Vec<String>,
    /// Supported signature algorithms
    pub supported_signatures: Vec<String>,
    /// Recommended TLS cipher list for TLS 1.2 and below
    pub recommended_cipher_list: String,
    /// Recommended TLS 1.3 ciphersuites
    pub recommended_tls13_ciphersuites: String,
    /// Recommended TLS groups (curves and key exchange mechanisms)
    pub recommended_groups: String,
}

/// Detected OpenSSL capabilities
///
/// This structure holds the capabilities detected from the OpenSSL environment.
/// If a field is None, it means the capability could not be detected and a default value should be used.
#[derive(Debug, Default, Clone)]
pub struct DetectedCapabilities {
    /// Detected TLS cipher list
    pub cipher_list: Option<String>,
    /// Detected TLS 1.3 ciphersuites
    pub tls13_ciphersuites: Option<String>,
    /// Detected TLS groups
    pub groups: Option<String>,
}

/// Cryptographic provider trait
///
/// This trait defines the interface for cryptographic providers.
/// Implementations include StandardProvider (using standard OpenSSL)
/// and OqsProvider (using OQS-OpenSSL with post-quantum support).
pub trait CryptoProvider: Send + Sync {
    /// Check if a certificate is a hybrid certificate
    fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool>;

    /// Get certificate subject
    fn get_cert_subject(&self, cert_path: &Path) -> Result<String>;

    /// Get certificate fingerprint
    fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String>;

    /// Load certificate
    fn load_cert(&self, cert_path: &Path) -> Result<X509>;

    /// Get the capabilities of this provider
    fn capabilities(&self) -> CryptoCapabilities;

    /// Get the name of this provider
    fn name(&self) -> &'static str;
}
