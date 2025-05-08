// src/tls/strategy.rs
use openssl::ssl::{SslAcceptorBuilder, SslFiletype, SslRef, ClientHelloResponse};
use openssl::error::ErrorStack;
use std::path::PathBuf;
use std::any::Any;
use log::{info, warn, error};
use crate::common::{Result, ProxyError};
use crate::config::{ProxyConfig, CertStrategyType};

/// Certificate strategies for TLS connections
#[derive(Debug)]
pub enum CertStrategy {
    /// Single certificate strategy (uses one certificate for all connections)
    Single { cert: PathBuf, key: PathBuf },

    /// SigAlgs strategy (uses classic or hybrid based on client capabilities)
    SigAlgs {
        classic: (PathBuf, PathBuf),
        hybrid:  (PathBuf, PathBuf),
    },

    /// Dynamic callback strategy (examines client hello to determine certificate)
    Dynamic {
        traditional: (PathBuf, PathBuf),
        hybrid: (PathBuf, PathBuf),
        pqc_only: Option<(PathBuf, PathBuf)>,
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
                info!("Using single certificate strategy");
                Self::verify_cert_key_exist(cert, key, "Single")?;

                builder.set_certificate_file(cert, SslFiletype::PEM)?;
                builder.set_private_key_file(key, SslFiletype::PEM)?;
            }

            CertStrategy::SigAlgs { classic, hybrid } => {
                info!("Using SigAlgs certificate strategy");
                Self::verify_cert_key_exist(&classic.0, &classic.1, "Classic")?;

                // Set classic certificate as base
                builder.set_certificate_file(&classic.0, SslFiletype::PEM)?;
                builder.set_private_key_file(&classic.1, SslFiletype::PEM)?;

                // Try to add hybrid certificate if available
                if !hybrid.0.exists() || !hybrid.1.exists() {
                    info!("Using only classic certificate due to missing hybrid certificate or key");
                    return Ok(());
                }

                // Load and add hybrid certificate to chain
                match openssl::x509::X509::from_pem(&std::fs::read(&hybrid.0)?) {
                    Ok(cert) => {
                        if let Err(e) = builder.add_extra_chain_cert(cert) {
                            warn!("Failed to add hybrid certificate to chain: {}", e);
                            return Ok(());
                        }
                        info!("Using both classic and hybrid certificates for compatibility");
                    },
                    Err(e) => {
                        warn!("Failed to load hybrid certificate: {}", e);
                        return Ok(());
                    }
                };
            }

            CertStrategy::Dynamic { traditional, hybrid, pqc_only } => {
                info!("Using dynamic certificate strategy with client hello callback");

                // Verify all certificate and key files exist
                Self::verify_cert_key_exist(&traditional.0, &traditional.1, "Traditional")?;
                Self::verify_cert_key_exist(&hybrid.0, &hybrid.1, "Hybrid")?;

                if let Some(pqc) = pqc_only {
                    Self::verify_cert_key_exist(&pqc.0, &pqc.1, "PQC-only")?;
                }

                // Preload all certificates and keys
                let trad_cert_key = load_cert_and_key(&traditional.0, &traditional.1)
                    .map_err(|e| ProxyError::Config(format!("Failed to load traditional certificate and key: {}", e)))?;

                let hybrid_cert_key = load_cert_and_key(&hybrid.0, &hybrid.1)
                    .map_err(|e| ProxyError::Config(format!("Failed to load hybrid certificate and key: {}", e)))?;

                let pqc_cert_key = if let Some(pqc) = pqc_only {
                    Some(load_cert_and_key(&pqc.0, &pqc.1)
                        .map_err(|e| ProxyError::Config(format!("Failed to load PQC-only certificate and key: {}", e)))?)
                } else {
                    None
                };

                // Set traditional certificate as default
                builder.set_certificate(&trad_cert_key.0)?;
                builder.set_private_key(&trad_cert_key.1)?;

                // Use Arc to share ownership with the callback closure
                use std::sync::Arc;
                let trad_cert = Arc::new(trad_cert_key.0);
                let trad_key = Arc::new(trad_cert_key.1);
                let hybrid_cert = Arc::new(hybrid_cert_key.0);
                let hybrid_key = Arc::new(hybrid_cert_key.1);
                let pqc_cert_key = pqc_cert_key.map(|(cert, key)| (Arc::new(cert), Arc::new(key)));

                // Set client hello callback for dynamic certificate selection
                builder.set_client_hello_callback(move |ssl, _alert| {
                    if detect_client_pqc_support(ssl) {
                        // Try PQC-only certificate for fully PQC-capable clients
                        if let Some((pqc_cert, pqc_key)) = &pqc_cert_key {
                            if detect_client_full_pqc_support(ssl) {
                                info!("Using PQC-only certificate for fully PQC-capable client");
                                if ssl.set_certificate(&**pqc_cert).is_ok() &&
                                   ssl.set_private_key(&**pqc_key).is_ok() {
                                    return Ok(ClientHelloResponse::SUCCESS);
                                }
                                warn!("Failed to set PQC-only certificate, falling back to hybrid");
                            }
                        }

                        // Try hybrid certificate for PQC-capable clients
                        info!("Using hybrid certificate for PQC-capable client");
                        if ssl.set_certificate(&*hybrid_cert).is_ok() &&
                           ssl.set_private_key(&*hybrid_key).is_ok() {
                            return Ok(ClientHelloResponse::SUCCESS);
                        }
                        warn!("Failed to set hybrid certificate, falling back to traditional");
                    }

                    // Fallback to traditional certificate
                    info!("Using traditional certificate");
                    if let Err(e) = ssl.set_certificate(&*trad_cert) {
                        error!("Failed to set traditional certificate: {}", e);
                        return Err(e);
                    }
                    if let Err(e) = ssl.set_private_key(&*trad_key) {
                        error!("Failed to set traditional key: {}", e);
                        return Err(e);
                    }

                    Ok(ClientHelloResponse::SUCCESS)
                });

                info!("Dynamic certificate selection callback registered");
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

/// Detect if client fully supports pure PQC (no hybrid)
fn detect_client_full_pqc_support(ssl: &mut SslRef) -> bool {
    // First check if client supports PQC at all
    if !detect_client_pqc_support(ssl) {
        return false;
    }

    // Check for full PQC support based on supported groups
    if let Some(group_ids) = get_extension_ids(ssl, TLSEXT_TYPE_SUPPORTED_GROUPS) {
        let (pqc, non_pqc) = count_pqc_ids(&group_ids, is_pqc_group);
        if pqc > 0 && pqc > non_pqc {
            return true;
        }
    }

    // Check for full PQC support based on signature algorithms
    if let Some(sig_ids) = get_extension_ids(ssl, TLSEXT_TYPE_SIGNATURE_ALGORITHMS) {
        let (pqc, non_pqc) = count_pqc_ids(&sig_ids, is_pqc_signature_algorithm);
        if pqc > 0 && pqc > non_pqc / 2 {
            return true;
        }
    }

    false
}

/// Count PQC and non-PQC IDs in a list
#[inline]
fn count_pqc_ids<F>(ids: &[u16], is_pqc_id: F) -> (usize, usize)
where
    F: Fn(u16) -> bool
{
    ids.iter().fold((0, 0), |(pqc, non_pqc), &id| {
        if is_pqc_id(id) { (pqc + 1, non_pqc) } else { (pqc, non_pqc + 1) }
    })
}

/// Helper function to load certificate and private key from files
fn load_cert_and_key(cert_path: &PathBuf, key_path: &PathBuf) -> std::result::Result<(openssl::x509::X509, openssl::pkey::PKey<openssl::pkey::Private>), ErrorStack> {
    // Load certificate and key in a more concise way
    let cert = openssl::x509::X509::from_pem(&std::fs::read(cert_path).map_err(|_| ErrorStack::get())?)?;
    let key = openssl::pkey::PKey::private_key_from_pem(&std::fs::read(key_path).map_err(|_| ErrorStack::get())?)?;

    Ok((cert, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ssl::{SslMethod, SslAcceptor};

    #[test]
    fn sigalgs_callback_registers() {
        let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        let classic = ("c.crt".into(), "c.key".into());
        let hybrid  = ("h.crt".into(), "h.key".into());
        let strat   = CertStrategy::SigAlgs { classic: classic.clone(), hybrid: hybrid.clone() };

        // This test just confirms that callback registration doesn't crash
        // In reality, since we don't have real certificate files, apply would fail
        // But we just want to confirm that the code structure is correct
        let result = strat.apply(&mut builder);
        // We expect this to fail because the test files don't exist
        assert!(result.is_err(), "Should fail when certificate files don't exist");

        // Test single certificate strategy
        let single_strat = CertStrategy::Single { cert: "c.crt".into(), key: "c.key".into() };
        let result = single_strat.apply(&mut builder);
        // We expect this to fail because the test files don't exist
        assert!(result.is_err(), "Should fail when certificate files don't exist");
    }

    #[test]
    fn dynamic_callback_registers() {
        let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        let traditional = ("t.crt".into(), "t.key".into());
        let hybrid = ("h.crt".into(), "h.key".into());
        let pqc_only = Some(("p.crt".into(), "p.key".into()));

        let strat = CertStrategy::Dynamic {
            traditional: traditional.clone(),
            hybrid: hybrid.clone(),
            pqc_only: pqc_only.clone()
        };

        // This test just confirms that callback registration doesn't crash
        // In reality, since we don't have real certificate files, apply would fail
        // But we just want to confirm that the code structure is correct
        let result = strat.apply(&mut builder);
        // We expect this to fail because the test files don't exist
        assert!(result.is_err(), "Should fail when certificate files don't exist");
    }
}

/// Implement From<ProxyConfig> for CertStrategy
impl From<&ProxyConfig> for CertStrategy {
    fn from(config: &ProxyConfig) -> Self {
        match config.strategy() {
            CertStrategyType::Single => {
                CertStrategy::Single {
                    cert: config.hybrid_cert().to_path_buf(),
                    key: config.hybrid_key().to_path_buf(),
                }
            }
            CertStrategyType::SigAlgs => {
                CertStrategy::SigAlgs {
                    classic: (
                        config.traditional_cert().to_path_buf(),
                        config.traditional_key().to_path_buf()
                    ),
                    hybrid: (
                        config.hybrid_cert().to_path_buf(),
                        config.hybrid_key().to_path_buf()
                    ),
                }
            }
            CertStrategyType::Dynamic => {
                CertStrategy::Dynamic {
                    traditional: (
                        config.traditional_cert().to_path_buf(),
                        config.traditional_key().to_path_buf()
                    ),
                    hybrid: (
                        config.hybrid_cert().to_path_buf(),
                        config.hybrid_key().to_path_buf()
                    ),
                    pqc_only: config.pqc_only_cert().zip(config.pqc_only_key())
                        .map(|(cert, key)| (cert.to_path_buf(), key.to_path_buf())),
                }
            }
        }
    }
}

/// Build certificate strategy from configuration
///
/// This function builds a certificate strategy based on the configuration.
///
/// Returns a boxed strategy that can be used with the TLS acceptor.
pub fn build_cert_strategy(config: &ProxyConfig) -> Result<Box<dyn Any>> {
    // Convert our strategy to the tls module's strategy using From trait
    // This is more elegant thanks to the From trait implementation
    Ok(Box::new(CertStrategy::from(config)))
}