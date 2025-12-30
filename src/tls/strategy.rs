// src/tls/strategy.rs
//!
//! Certificate strategy for TLS connections.
//!
//! The proxy automatically determines the strategy based on configuration:
//! - Single mode: Only primary certificate configured
//! - Dynamic mode: Both primary and fallback certificates configured

use openssl::ssl::{SslAcceptorBuilder, SslFiletype, SslRef, ClientHelloResponse};
use openssl::error::ErrorStack;
use std::path::PathBuf;
use std::any::Any;
use log::{info, warn, error};
use crate::common::{Result, ProxyError};
use crate::config::ProxyConfig;

/// Certificate strategies for TLS connections
#[derive(Debug)]
pub enum CertStrategy {
    /// Single certificate strategy (uses one certificate for all connections)
    Single { 
        cert: PathBuf, 
        key: PathBuf,
    },

    /// Dynamic callback strategy (examines client hello to determine certificate)
    /// Automatically selects between primary (PQC/hybrid) and fallback (traditional)
    Dynamic {
        /// Primary certificate (typically hybrid/PQC)
        primary: (PathBuf, PathBuf),
        /// Fallback certificate for non-PQC clients (traditional RSA/ECDSA)
        fallback: (PathBuf, PathBuf),
    },
}

impl CertStrategy {
    /// Verify that certificate and key files exist
    fn verify_cert_key_exist(cert: &PathBuf, key: &PathBuf, name: &str) -> Result<()> {
        if !cert.exists() {
            return Err(ProxyError::Config(format!("{} certificate file does not exist: {:?}", name, cert)));
        }
        if !key.exists() {
            return Err(ProxyError::Config(format!("{} key file does not exist: {:?}", name, key)));
        }
        Ok(())
    }

    /// Apply the chosen strategy to the OpenSSL builder.
    pub fn apply(&self, builder: &mut SslAcceptorBuilder) -> Result<()> {
        match self {
            CertStrategy::Single { cert, key } => {
                info!("Using single certificate mode");
                Self::verify_cert_key_exist(cert, key, "Primary")?;

                builder.set_certificate_file(cert, SslFiletype::PEM)?;
                builder.set_private_key_file(key, SslFiletype::PEM)?;
            }

            CertStrategy::Dynamic { primary, fallback } => {
                info!("Using dynamic certificate mode (auto-select based on client capabilities)");

                // Verify all certificate and key files exist
                Self::verify_cert_key_exist(&primary.0, &primary.1, "Primary")?;
                Self::verify_cert_key_exist(&fallback.0, &fallback.1, "Fallback")?;

                // Preload all certificates and keys
                let primary_cert_key = load_cert_and_key(&primary.0, &primary.1)
                    .map_err(|e| ProxyError::Config(format!("Failed to load primary certificate: {}", e)))?;

                let fallback_cert_key = load_cert_and_key(&fallback.0, &fallback.1)
                    .map_err(|e| ProxyError::Config(format!("Failed to load fallback certificate: {}", e)))?;

                // Set fallback certificate as default (for non-PQC clients)
                builder.set_certificate(&fallback_cert_key.0)?;
                builder.set_private_key(&fallback_cert_key.1)?;

                // Use Arc to share ownership with the callback closure
                use std::sync::Arc;
                let primary_cert = Arc::new(primary_cert_key.0);
                let primary_key = Arc::new(primary_cert_key.1);
                let fallback_cert = Arc::new(fallback_cert_key.0);
                let fallback_key = Arc::new(fallback_cert_key.1);

                // Set client hello callback for dynamic certificate selection
                builder.set_client_hello_callback(move |ssl, _alert| {
                    if detect_client_pqc_support(ssl) {
                        // Use primary (PQC/hybrid) certificate for PQC-capable clients
                        info!("Client supports PQC, using primary certificate");
                        if ssl.set_certificate(&*primary_cert).is_ok() &&
                           ssl.set_private_key(&*primary_key).is_ok() {
                            return Ok(ClientHelloResponse::SUCCESS);
                        }
                        warn!("Failed to set primary certificate, falling back");
                    }

                    // Use fallback (traditional) certificate
                    info!("Using fallback certificate for traditional client");
                    if let Err(e) = ssl.set_certificate(&*fallback_cert) {
                        error!("Failed to set fallback certificate: {}", e);
                        return Err(e);
                    }
                    if let Err(e) = ssl.set_private_key(&*fallback_key) {
                        error!("Failed to set fallback key: {}", e);
                        return Err(e);
                    }

                    Ok(ClientHelloResponse::SUCCESS)
                });

                info!("Dynamic certificate selection enabled");
            }
        }

        Ok(())
    }
}

use openssl_sys::SSL_client_hello_get0_ext;
use std::slice;
use foreign_types_shared::ForeignTypeRef;

// TLS extension IDs
const TLSEXT_TYPE_SUPPORTED_GROUPS: u32 = 10;
const TLSEXT_TYPE_SIGNATURE_ALGORITHMS: u32 = 13;

// PQC group and signature algorithm IDs
const PQC_GROUP_RANGES: [(u16, u16); 2] = [(0x0600, 0x06FF), (0x2F80, 0x2FFF)];
const PQC_SIG_ALG_RANGE: (u16, u16) = (0x0900, 0x09FF);

// Key PQC identifiers
const X25519MLKEM768: u16 = 0x2F80;
const DILITHIUM2: u16 = 0x0901;

/// Detect if client supports post-quantum cryptography
fn detect_client_pqc_support(ssl: &mut SslRef) -> bool {
    // Require TLS 1.3 for PQC support
    if !ssl.client_hello_ciphers().map_or(false, |c| !c.is_empty() && c.iter().any(|&c| c == 0x13)) {
        return false;
    }

    // Check for PQC support in either groups or signature algorithms
    has_pqc_extension(ssl, TLSEXT_TYPE_SUPPORTED_GROUPS, is_pqc_group) ||
    has_pqc_extension(ssl, TLSEXT_TYPE_SIGNATURE_ALGORITHMS, is_pqc_signature_algorithm)
}

/// Check if client has PQC support in a specific extension
#[inline]
fn has_pqc_extension<F>(ssl: &mut SslRef, ext_type: u32, is_pqc_id: F) -> bool
where
    F: Fn(u16) -> bool
{
    get_extension_ids(ssl, ext_type).map_or(false, |ids| ids.iter().any(|&id| is_pqc_id(id)))
}

/// Get IDs from a TLS extension
fn get_extension_ids(ssl: &mut SslRef, extension_type: u32) -> Option<Vec<u16>> {
    unsafe {
        let ssl_ptr = ssl.as_ptr();
        let mut data: *const u8 = std::ptr::null();
        let mut len: usize = 0;

        // Get extension data via FFI
        if SSL_client_hello_get0_ext(ssl_ptr, extension_type, &mut data, &mut len) != 1 ||
           data.is_null() || len < 2 {
            return None;
        }

        // Parse extension data
        let ext_data = slice::from_raw_parts(data, len);
        let list_len = ((ext_data[0] as usize) << 8) | (ext_data[1] as usize);

        if list_len + 2 > len {
            return None;
        }

        // Extract IDs (each ID is 2 bytes)
        let mut ids = Vec::with_capacity(list_len / 2);
        for i in (0..list_len).step_by(2) {
            if i + 3 < len {
                ids.push(((ext_data[i + 2] as u16) << 8) | (ext_data[i + 3] as u16));
            }
        }

        Some(ids)
    }
}

/// Check if a group ID represents a PQC or hybrid group
#[inline]
fn is_pqc_group(id: u16) -> bool {
    id == X25519MLKEM768 || // Most common hybrid group
    PQC_GROUP_RANGES.iter().any(|&(start, end)| id >= start && id <= end)
}

/// Check if a signature algorithm ID represents a PQC signature algorithm
#[inline]
fn is_pqc_signature_algorithm(id: u16) -> bool {
    id == DILITHIUM2 || // Most common PQC signature algorithm
    (id >= PQC_SIG_ALG_RANGE.0 && id <= PQC_SIG_ALG_RANGE.1)
}

/// Helper function to load certificate and private key from files
fn load_cert_and_key(cert_path: &PathBuf, key_path: &PathBuf) -> std::result::Result<(openssl::x509::X509, openssl::pkey::PKey<openssl::pkey::Private>), ErrorStack> {
    let cert = openssl::x509::X509::from_pem(&std::fs::read(cert_path).map_err(|_| ErrorStack::get())?)?;
    let key = openssl::pkey::PKey::private_key_from_pem(&std::fs::read(key_path).map_err(|_| ErrorStack::get())?)?;

    Ok((cert, key))
}

/// Build certificate strategy from configuration
///
/// Automatically determines the strategy based on configuration:
/// - If fallback certificates are configured → Dynamic mode
/// - Otherwise → Single mode
impl From<&ProxyConfig> for CertStrategy {
    fn from(config: &ProxyConfig) -> Self {
        if config.has_fallback() {
            // Dynamic mode: auto-select based on client capabilities
            CertStrategy::Dynamic {
                primary: (
                    config.cert().to_path_buf(),
                    config.key().to_path_buf(),
                ),
                fallback: (
                    config.fallback_cert().unwrap().to_path_buf(),
                    config.fallback_key().unwrap().to_path_buf(),
                ),
            }
        } else {
            // Single mode: use primary certificate for all clients
            CertStrategy::Single {
                cert: config.cert().to_path_buf(),
                key: config.key().to_path_buf(),
            }
        }
    }
}

/// Build certificate strategy from configuration
///
/// Returns a boxed strategy that can be used with the TLS acceptor.
pub fn build_cert_strategy(config: &ProxyConfig) -> Result<Box<dyn Any>> {
    Ok(Box::new(CertStrategy::from(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ssl::{SslMethod, SslAcceptor};

    #[test]
    fn single_strategy_requires_cert_files() {
        let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        let strat = CertStrategy::Single { 
            cert: "nonexistent.crt".into(), 
            key: "nonexistent.key".into(),
        };

        let result = strat.apply(&mut builder);
        assert!(result.is_err(), "Should fail when certificate files don't exist");
    }

    #[test]
    fn dynamic_strategy_requires_all_cert_files() {
        let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        let strat = CertStrategy::Dynamic {
            primary: ("primary.crt".into(), "primary.key".into()),
            fallback: ("fallback.crt".into(), "fallback.key".into()),
        };

        let result = strat.apply(&mut builder);
        assert!(result.is_err(), "Should fail when certificate files don't exist");
    }

    #[test]
    fn test_strategy_from_config_single() {
        // Create a config without fallback (Single mode)
        let mut config = crate::config::ProxyConfig::default();
        config.values.cert = Some("certs/hybrid/server.crt".into());
        config.values.key = Some("certs/hybrid/server.key".into());

        let strategy = CertStrategy::from(&config);
        
        match strategy {
            CertStrategy::Single { cert, key } => {
                assert_eq!(cert.to_string_lossy(), "certs/hybrid/server.crt");
                assert_eq!(key.to_string_lossy(), "certs/hybrid/server.key");
            }
            _ => panic!("Expected Single strategy"),
        }
    }

    #[test]
    fn test_strategy_from_config_dynamic() {
        // Create a config with fallback (Dynamic mode)
        let mut config = crate::config::ProxyConfig::default();
        config.values.cert = Some("certs/hybrid/server.crt".into());
        config.values.key = Some("certs/hybrid/server.key".into());
        config.values.fallback_cert = Some("certs/traditional/server.crt".into());
        config.values.fallback_key = Some("certs/traditional/server.key".into());

        let strategy = CertStrategy::from(&config);
        
        match strategy {
            CertStrategy::Dynamic { primary, fallback } => {
                assert_eq!(primary.0.to_string_lossy(), "certs/hybrid/server.crt");
                assert_eq!(fallback.0.to_string_lossy(), "certs/traditional/server.crt");
            }
            _ => panic!("Expected Dynamic strategy"),
        }
    }
}
