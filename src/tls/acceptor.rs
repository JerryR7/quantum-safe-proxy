//! TLS acceptor creation with hybrid certificate support

use log::{debug, info};
use openssl::ssl::{SslAcceptor, SslVerifyMode, SslMethod};
use openssl::stack::Stack;
use openssl::x509::{X509, X509Name};
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
    let provider_supports_pqc = capabilities.supports_pqc;

    // Log the provider and its capabilities
    info!("Using crypto provider: {} (PQC support: {})",
          provider.name(),
          if provider_supports_pqc { "available" } else { "not available" });
    debug!("Provider supports PQC: {}", provider_supports_pqc);

    // We can't directly set verification mode on the SslContext from our provider,
    // So we'll create a new SslAcceptor and configure it

    debug!("Creating TLS acceptor with the following parameters:");
    debug!("  CA cert path: {:?}", ca_cert_path);
    debug!("  Client cert mode: {:?}", client_cert_mode);
    debug!("  Strategy: {:?}", strategy);

    // Create a new SslAcceptor with the appropriate settings
    let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    debug!("Created SslAcceptor with mozilla_intermediate_v5 profile");

    // Apply the certificate strategy
    strategy.apply(&mut acceptor)?;
    debug!("Applied certificate strategy");

    // We no longer hardcode supported signature algorithms and groups, letting OpenSSL choose automatically
    // This ensures we use algorithms and groups supported by the OpenSSL version
    debug!("Using OpenSSL's default signature algorithms and groups");

    // Set TLS 1.3 cipher suites
    // These are standard TLS 1.3 cipher suites that should be supported by all TLS 1.3 implementations
    let ciphersuites = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256";
    debug!("Setting supported TLS 1.3 cipher suites: {}", ciphersuites);
    acceptor.set_ciphersuites(ciphersuites)?;

    // Set verification mode based on client certificate mode
    match client_cert_mode {
        ClientCertMode::Required => {
            let verify_mode = SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT;
            info!("Client certificates required (will be verified)");
            acceptor.set_verify(verify_mode);

            // Set verification depth to ensure the entire certificate chain is validated
            acceptor.set_verify_depth(10);
        },
        ClientCertMode::Optional => {
            let verify_mode = SslVerifyMode::PEER;
            info!("Client certificates optional (will be verified if provided)");
            acceptor.set_verify(verify_mode);

            // Set verification depth for optional mode as well
            acceptor.set_verify_depth(10);
        },
        ClientCertMode::None => {
            let verify_mode = SslVerifyMode::NONE;
            info!("Client certificates not required (no verification)");
            acceptor.set_verify(verify_mode);
        },
    }

    // Configure CA certificates and client CA list when client certificates are needed
    if !matches!(client_cert_mode, ClientCertMode::None) {
        // 1) Set CA file for certificate verification
        acceptor.set_ca_file(ca_cert_path)?;
        debug!("Set CA certificate file: {:?}", ca_cert_path);

        // 2) Read all certificates from the CA file (may contain multiple PEM certificates)
        let ca_pem = std::fs::read(ca_cert_path)?;
        let ca_chain = X509::stack_from_pem(&ca_pem)?;
        debug!("Loaded {} CA certificate(s) from file", ca_chain.len());

        let store = acceptor.cert_store_mut();

        // 3) Build client CA list (the list of acceptable CAs sent to the client)
        let mut ca_names = Stack::<X509Name>::new()?;

        for ca in &ca_chain {
            // Add each CA to the trust store for verification
            store.add_cert(ca.to_owned())?;

            // Check if this is a self-signed root certificate
            // Compare the DER-encoded bytes since X509NameRef doesn't implement PartialEq
            let issuer_der = ca.issuer_name().to_der()?;
            let subject_der = ca.subject_name().to_der()?;
            let is_self_signed = issuer_der == subject_der;

            if !is_self_signed {
                // Only add intermediate CAs to the client CA list (not root CAs)
                ca_names.push(ca.subject_name().to_owned()?)?;
                debug!("Added intermediate CA to client list: {:?}", ca.subject_name());
            } else {
                debug!("Skipped self-signed root CA from client list: {:?}", ca.subject_name());
            }
        }

        // 4) Set the client CA list (this fixes "No client certificate CA names sent")
        acceptor.set_client_ca_list(ca_names);
        info!("Configured client CA list from {:?}", ca_cert_path);
    }

    Ok(acceptor.build())
}