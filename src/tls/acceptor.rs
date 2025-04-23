//! TLS acceptor creation with hybrid certificate support

use log::{debug, info};
use openssl::ssl::{SslAcceptor, SslVerifyMode, SslMethod};
use std::path::Path;

use crate::common::Result;
use crate::config::ClientCertMode;
use crate::crypto::get_provider;
use crate::tls::strategy::CertStrategy;

/// Create TLS acceptor with hybrid certificate support
///
/// # Example
///
/// ```no_run
/// # use std::path::Path;
/// # use quantum_safe_proxy::tls::create_tls_acceptor;
/// # use quantum_safe_proxy::config::ClientCertMode;
/// # use quantum_safe_proxy::tls::strategy::CertStrategy;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let strategy = CertStrategy::Single {
///     cert: Path::new("certs/openssl35/server/server.crt").to_path_buf(),
///     key: Path::new("certs/openssl35/server/server.key").to_path_buf(),
/// };
/// let acceptor = create_tls_acceptor(
///     Path::new("certs/openssl35/ca/ca.crt"),
///     &ClientCertMode::Required,
///     strategy
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_tls_acceptor(
    ca_cert_path: &Path,
    client_cert_mode: &ClientCertMode,
    strategy: CertStrategy,
) -> Result<SslAcceptor> {
    // Get the global crypto provider
    let provider = get_provider();
    let capabilities = provider.capabilities();

    // Log the provider and its capabilities
    info!("Using crypto provider: {} (PQC support: {})",
          provider.name(),
          if capabilities.supports_pqc { "available" } else { "not available" });
    debug!("Provider supports PQC: {}", capabilities.supports_pqc);

    // We can't directly set verification mode on the SslContext from our provider,
    // So we'll create a new SslAcceptor and configure it

    // Create a new SslAcceptor with the appropriate settings
    let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;

    // Apply the certificate strategy
    strategy.apply(&mut acceptor)?;

    // Apply the CA certificate
    acceptor.set_ca_file(ca_cert_path)?;

    // Set verification mode based on client certificate mode
    match client_cert_mode {
        ClientCertMode::Required => {
            info!("Client certificates required (will be verified)");
            acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        },
        ClientCertMode::Optional => {
            info!("Client certificates optional (will be verified if provided)");
            acceptor.set_verify(SslVerifyMode::PEER);
        },
        ClientCertMode::None => {
            info!("Client certificates not required (no verification)");
            acceptor.set_verify(SslVerifyMode::NONE);
        },
    }

    Ok(acceptor.build())
}
