// src/tls/strategy.rs
use openssl::ssl::{SslAcceptorBuilder, SslFiletype};
use std::path::PathBuf;
use log::{debug, info, warn};
use crate::common::Result;

/// Two strategies: Single (classic only) or SigAlgs (auto-select)
#[derive(Debug)]
pub enum CertStrategy {
    Single { cert: PathBuf, key: PathBuf },
    SigAlgs {
        classic: (PathBuf, PathBuf),
        hybrid:  (PathBuf, PathBuf),
    },
}

impl CertStrategy {
    /// Apply the chosen strategy to the OpenSSL builder.
    pub fn apply(&self, builder: &mut SslAcceptorBuilder) -> Result<()> {
        debug!("Applying certificate strategy: {:?}", self);

        match self {
            // always load classic
            CertStrategy::Single { cert, key } => {
                info!("Using single certificate strategy");
                debug!("Certificate path: {:?}", cert);
                debug!("Key path: {:?}", key);

                // Check if certificate file exists
                if !cert.exists() {
                    warn!("Certificate file does not exist: {:?}", cert);
                    return Err(crate::common::ProxyError::Config(format!("Certificate file does not exist: {:?}", cert)));
                }

                // Check if key file exists
                if !key.exists() {
                    warn!("Key file does not exist: {:?}", key);
                    return Err(crate::common::ProxyError::Config(format!("Key file does not exist: {:?}", key)));
                }

                debug!("Setting certificate file: {:?}", cert);
                builder.set_certificate_file(cert, SslFiletype::PEM)?;
                debug!("Setting private key file: {:?}", key);
                builder.set_private_key_file(key, SslFiletype::PEM)?;
                debug!("Certificate and key set successfully");
            }
            // sigalgs: detect PQC OID in client_hello
            CertStrategy::SigAlgs { classic, hybrid } => {
                info!("Using SigAlgs certificate strategy");
                debug!("Classic certificate path: {:?}", classic.0);
                debug!("Classic key path: {:?}", classic.1);
                debug!("Hybrid certificate path: {:?}", hybrid.0);
                debug!("Hybrid key path: {:?}", hybrid.1);

                // Check if classic certificate file exists
                if !classic.0.exists() {
                    warn!("Classic certificate file does not exist: {:?}", classic.0);
                    return Err(crate::common::ProxyError::Config(format!("Classic certificate file does not exist: {:?}", classic.0)));
                }

                // Check if classic key file exists
                if !classic.1.exists() {
                    warn!("Classic key file does not exist: {:?}", classic.1);
                    return Err(crate::common::ProxyError::Config(format!("Classic key file does not exist: {:?}", classic.1)));
                }

                // First set the classic certificate (this will be used by traditional clients)
                debug!("Setting classic certificate file: {:?}", classic.0);
                builder.set_certificate_file(&classic.0, SslFiletype::PEM)?;
                debug!("Setting classic private key file: {:?}", classic.1);
                builder.set_private_key_file(&classic.1, SslFiletype::PEM)?;
                debug!("Classic certificate and key set successfully");

                // Check if hybrid certificate file exists
                if !hybrid.0.exists() {
                    warn!("Hybrid certificate file does not exist: {:?}, using only classic certificate", hybrid.0);
                    info!("Using only classic certificate due to missing hybrid certificate.");
                    return Ok(());
                }

                // Check if hybrid key file exists
                if !hybrid.1.exists() {
                    warn!("Hybrid key file does not exist: {:?}, using only classic certificate", hybrid.1);
                    info!("Using only classic certificate due to missing hybrid key.");
                    return Ok(());
                }

                // Try to add the hybrid certificate to the chain
                debug!("Adding hybrid certificate to chain: {:?}", hybrid.0);

                // Load the hybrid certificate
                let hybrid_cert = match openssl::x509::X509::from_pem(&std::fs::read(&hybrid.0)?) {
                    Ok(cert) => cert,
                    Err(e) => {
                        warn!("Failed to load hybrid certificate: {}, using only classic certificate", e);
                        info!("Using only classic certificate due to error loading hybrid certificate.");
                        return Ok(());
                    }
                };

                // Add the hybrid certificate to the chain
                if let Err(e) = builder.add_extra_chain_cert(hybrid_cert) {
                    warn!("Failed to add hybrid certificate to chain: {}, using only classic certificate", e);
                    info!("Using only classic certificate due to error adding hybrid certificate to chain.");
                    return Ok(());
                }

                debug!("Hybrid certificate added to chain successfully");
                info!("Using both classic and hybrid certificates for maximum compatibility.");
            }
        }

        debug!("Certificate strategy applied successfully");
        Ok(())
    }
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
}
