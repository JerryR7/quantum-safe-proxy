//! TLS acceptor creation with hybrid certificate support

use log::{debug, info};
use openssl::ssl::{SslAcceptor, SslFiletype, SslVerifyMode, SslMethod};
use std::path::Path;

use crate::common::Result;
use crate::config::ClientCertMode;
use crate::crypto::get_provider;

/// Create TLS acceptor with hybrid certificate support
///
/// # Example
///
/// ```no_run
/// # use std::path::Path;
/// # use quantum_safe_proxy::tls::create_tls_acceptor;
/// # use quantum_safe_proxy::config::ClientCertMode;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let acceptor = create_tls_acceptor(
///     Path::new("certs/openssl35/server/server.crt"),
///     Path::new("certs/openssl35/server/server.key"),
///     Path::new("certs/openssl35/ca/ca.crt"),
///     &ClientCertMode::Required
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_tls_acceptor(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
    client_cert_mode: &ClientCertMode,
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

    // Apply the certificate and key
    acceptor.set_certificate_file(cert_path, SslFiletype::PEM)?;
    acceptor.set_private_key_file(key_path, SslFiletype::PEM)?;
    acceptor.check_private_key()?;

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
