//! Cryptography module
//!
//! This module provides cryptographic functionality for the quantum-safe proxy,
//! using OpenSSL 3.5+ for post-quantum cryptography support.

mod openssl;
mod capabilities;
pub mod environment;
pub mod loader;



// OpenSSL type aliases
pub use ::openssl::ssl::SslContext;
pub use ::openssl::x509::X509;

// Public exports
pub use openssl::OpenSSLProvider as CryptoProvider;
pub use capabilities::{is_openssl35_available, is_pqc_available, get_openssl_version};
pub use capabilities::{get_supported_pq_algorithms, get_supported_signature_algorithms};
pub use capabilities::{get_recommended_cipher_list, get_recommended_tls13_ciphersuites, get_recommended_groups};
pub use environment::{check_environment, diagnose_environment, EnvironmentInfo, EnvironmentIssue, IssueSeverity};
pub use loader::initialize_openssl;

// Global provider accessor
pub fn get_provider() -> &'static CryptoProvider {
    CryptoProvider::global()
}

/// Certificate type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateType {
    /// Traditional certificate (RSA, ECDSA, etc.)
    Traditional,

    /// Hybrid certificate (traditional + post-quantum)
    Hybrid,

    /// Pure post-quantum certificate
    PostQuantum,
}

/// Cryptographic capabilities structure
#[derive(Debug, Clone)]
pub struct CryptoCapabilities {
    /// Whether the provider supports post-quantum cryptography
    pub supports_pqc: bool,

    /// OpenSSL version string
    pub openssl_version: String,

    /// Supported post-quantum algorithms
    pub supported_pq_algorithms: Vec<String>,

    /// Recommended TLS cipher list
    pub recommended_cipher_list: String,

    /// Recommended TLS 1.3 ciphersuites
    pub recommended_tls13_ciphersuites: String,

    /// Recommended TLS groups
    pub recommended_groups: String,
}
