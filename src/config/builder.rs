//! Certificate strategy builder functionality
//!
//! This module provides functionality for building certificate strategies.

use crate::common::Result;
use crate::config::{ProxyConfig, CertStrategyType};
use crate::tls::strategy::CertStrategy;

/// Trait for building certificate strategies
pub trait CertificateStrategyBuilder {
    /// Build the certificate strategy based on configuration
    fn build_cert_strategy(&self) -> Result<CertStrategy>;
}

impl CertificateStrategyBuilder for ProxyConfig {
    /// Build the certificate strategy based on configuration.
    ///
    /// Uses the strategy field to determine which certificate strategy to use.
    /// Optimized to avoid unnecessary clones.
    fn build_cert_strategy(&self) -> Result<CertStrategy> {
        // Create the appropriate certificate strategy based on the configuration
        Ok(match self.strategy {
            CertStrategyType::Single => {
                // Use hybrid certificate for single strategy
                CertStrategy::Single {
                    cert: self.hybrid_cert.clone(),
                    key: self.hybrid_key.clone(),
                }
            },
            CertStrategyType::SigAlgs => {
                // Use SigAlgs strategy with traditional and hybrid certificates
                CertStrategy::SigAlgs {
                    classic: (self.traditional_cert.clone(), self.traditional_key.clone()),
                    hybrid: (self.hybrid_cert.clone(), self.hybrid_key.clone()),
                }
            },
            CertStrategyType::Dynamic => {
                // Use Dynamic strategy with all certificate types
                CertStrategy::Dynamic {
                    traditional: (self.traditional_cert.clone(), self.traditional_key.clone()),
                    hybrid: (self.hybrid_cert.clone(), self.hybrid_key.clone()),
                    pqc_only: match (&self.pqc_only_cert, &self.pqc_only_key) {
                        (Some(cert), Some(key)) => Some((cert.clone(), key.clone())),
                        _ => None,
                    },
                }
            },
        })
    }
}
