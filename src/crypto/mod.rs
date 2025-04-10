//! Cryptography module
//!
//! This module provides cryptographic functionality for the quantum-safe proxy,
//! including support for both standard OpenSSL and OQS-OpenSSL providers.

pub mod provider;

// Re-export commonly used types and functions
pub use provider::{CryptoProvider, create_provider, ProviderType, CryptoCapabilities, CertificateType};
