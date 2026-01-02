//! Connection handler module
//!
//! This module handles individual client connections.

use log::{info, error, debug};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_openssl::SslStream;

use crate::config::{ProxyConfig, ClientCertMode, get_connection_timeout};
use crate::protocol::{ProtocolDetector, TlsDetector, DetectionResult};
use crate::admin::CryptoMode;

use crate::common::{ProxyError, Result};
use super::forwarder::proxy_data;

/// Handle a single client connection
///
/// # Parameters
///
/// * `client_stream` - Client TCP stream
/// * `target_addr` - Target service address
/// * `tls_acceptor` - TLS acceptor
/// * `config` - Proxy configuration
///
/// # Returns
///
/// Returns `Ok(())` if handling is successful, otherwise returns an error.
/// Check if connection uses TLS protocol
///
/// Determines if connection uses TLS by examining the first few bytes using the protocol detector.
/// If not a TLS connection, sends TCP RST to immediately close the connection.
/// Uses a non-blocking approach similar to NGINX.
async fn ensure_tls_connection(stream: TcpStream) -> Result<TcpStream> {
    // Enable TCP_NODELAY for faster response
    stream.set_nodelay(true).map_err(ProxyError::Io)?;

    // Create TLS detector
    let detector = TlsDetector::default();
    let mut stream_clone = stream;

    // Detect protocol with 100ms timeout (balanced for security and compatibility)
    match detector.detect(&mut stream_clone, 100).await? {
        DetectionResult::Tls => {
            debug!("TLS connection detected, continuing handshake");
            Ok(stream_clone)
        },
        DetectionResult::NonTls(reason) => {
            info!("Non-TLS connection detected: {}", reason);
            send_tcp_rst(&stream_clone)?;
            Err(ProxyError::NonTlsConnection(reason))
        },
        DetectionResult::NeedMoreData => {
            debug!("Not enough data to determine protocol, assuming non-TLS connection");
            send_tcp_rst(&stream_clone)?;
            Err(ProxyError::NonTlsConnection("Not enough data to determine protocol".to_string()))
        }
    }
}

/// Send TCP RST packet to immediately close connection
fn send_tcp_rst(stream: &TcpStream) -> Result<()> {
    // Setting SO_LINGER to 0 will send TCP RST when closing
    stream.set_linger(Some(Duration::from_secs(0)))
        .map_err(|e| {
            debug!("Failed to set TCP RST option: {}", e);
            ProxyError::Io(e)
        })
}

/// Classify TLS connection crypto mode based on cipher suite
///
/// Implements Constitution Principle IV: Cryptographic Mode Classification
///
/// # Classification Logic
///
/// - **Hybrid**: Cipher contains both PQC (MLKEM/KYBER) and classical (X25519/P256) components
/// - **PQC**: Cipher contains only PQC components (future support)
/// - **Classical**: Standard ECDHE, RSA, or other non-PQC ciphers
///
/// # Parameters
///
/// * `ssl` - OpenSSL connection reference after successful handshake
///
/// # Returns
///
/// Returns the classified cryptographic mode
fn classify_crypto_mode(ssl: &openssl::ssl::SslRef) -> CryptoMode {
    let cipher_name = ssl.current_cipher()
        .map(|c| c.name())
        .unwrap_or("UNKNOWN");

    debug!("Classifying cipher: {}", cipher_name);

    // Check for PQC algorithms (MLKEM, KYBER)
    let has_pqc = cipher_name.contains("MLKEM") || cipher_name.contains("KYBER");

    // Check for classical key exchange (X25519, P256, ECDHE)
    let has_classical = cipher_name.contains("X25519")
        || cipher_name.contains("P256")
        || cipher_name.contains("P384")
        || cipher_name.contains("P521")
        || cipher_name.contains("ECDHE");

    if has_pqc {
        if has_classical {
            // Hybrid: Contains both PQC and classical components
            // Example: TLS_AES_256_GCM_SHA384 with X25519MLKEM768
            CryptoMode::Hybrid
        } else {
            // Pure PQC (if ever supported by crypto stack)
            CryptoMode::Pqc
        }
    } else {
        // Classical TLS only (ECDHE, RSA, DHE, etc.)
        CryptoMode::Classical
    }
}

pub async fn handle_connection(
    client_stream: TcpStream,
    target_addr: SocketAddr,
    tls_acceptor: Arc<SslAcceptor>,
    config: &ProxyConfig,
) -> Result<()> {
    // First ensure this is a TLS connection
    let client_stream = ensure_tls_connection(client_stream).await?;

    // Setup TLS with client verification mode
    let mut ssl = openssl::ssl::Ssl::new(tls_acceptor.context()).map_err(ProxyError::Ssl)?;
    ssl.set_verify(match config.client_cert_mode() {
        ClientCertMode::Required => openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
        ClientCertMode::Optional => openssl::ssl::SslVerifyMode::PEER,
        ClientCertMode::None => openssl::ssl::SslVerifyMode::NONE,
    });

    // Create and accept TLS stream
    let mut stream = Box::pin(SslStream::new(ssl, client_stream).map_err(ProxyError::Ssl)?);

    // Perform TLS handshake with error handling
    if let Err(e) = stream.as_mut().accept().await {
        // Log error details if error logging is enabled
        if log::log_enabled!(log::Level::Error) {
            let ssl_error = stream.as_ref().get_ref().ssl().verify_result();
            error!("TLS handshake failed: {e}, verify result: {ssl_error}");

            // Extract OpenSSL error code if present
            e.to_string().strip_prefix("error:").and_then(|s| s.find(':'))
                .map(|code_end| error!("OpenSSL error code: {}", &e.to_string()[6..6+code_end]));

            // Emit structured telemetry for handshake failure
            error!("security.handshake.result=failure security.handshake.error={}", e);
        }
        return Err(ProxyError::TlsHandshake(e.to_string()));
    }

    debug!("TLS handshake successful");

    // Classify cryptographic mode (Constitution Principle IV - MANDATORY)
    let ssl = stream.as_ref().get_ref().ssl();
    let crypto_mode = classify_crypto_mode(ssl);
    let tls_version = ssl.version_str();
    let cipher_name = ssl.current_cipher().map_or("UNKNOWN", |c| c.name());

    // Emit telemetry for security observability (Principle VI)
    info!(
        "Established secure connection | crypto_mode={:?} tls_version={} cipher={}",
        crypto_mode, tls_version, cipher_name
    );

    // Structured logging for metrics collection
    if log::log_enabled!(log::Level::Debug) {
        debug!(
            "security.crypto_mode={:?} security.tls.version={} security.cipher={} security.handshake.result=success",
            crypto_mode, tls_version, cipher_name
        );
        debug!("TLS SNI: {}", ssl.servername(openssl::ssl::NameType::HOST_NAME).unwrap_or("None"));

        // Log client certificate if present and info logging is enabled
        if log::log_enabled!(log::Level::Info) {
            ssl.peer_certificate()
                .map(|cert| info!("Client certificate subject: {:?}", cert.subject_name()));
        }
    }

    // Connect to target with timeout
    let timeout_secs = get_connection_timeout();
    let target_stream = timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(target_addr)
    )
    .await
    .map_err(|_| ProxyError::ConnectionTimeout(timeout_secs))?
    .map_err(ProxyError::Io)?;

    // Forward data between client and target
    proxy_data(stream, target_stream, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    // Helper function to create a connected pair of TCP streams
    async fn create_tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_connect = tokio::spawn(async move {
            TcpStream::connect(addr).await.unwrap()
        });

        let (server, _) = listener.accept().await.unwrap();
        let client = client_connect.await.unwrap();

        (client, server)
    }

    #[tokio::test]
    async fn test_ensure_tls_connection_with_tls_data() {
        let (mut client, server) = create_tcp_pair().await;

        // Simulate TLS ClientHello
        let tls_client_hello = [
            0x16, 0x03, 0x03, 0x00, 0x31, // TLS record header (type, version, length)
            0x01, 0x00, 0x00, 0x2d, 0x03, 0x03, // Handshake header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Random (truncated)
        ];

        // Send TLS ClientHello from client to server
        client.write_all(&tls_client_hello).await.unwrap();

        // Test ensure_tls_connection
        let result = ensure_tls_connection(server).await;
        assert!(result.is_ok(), "Should accept TLS connection");
    }

    #[tokio::test]
    async fn test_ensure_tls_connection_with_non_tls_data() {
        let (mut client, server) = create_tcp_pair().await;

        // Simulate HTTP request
        let http_request = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";

        // Send HTTP request from client to server
        client.write_all(http_request).await.unwrap();

        // Test ensure_tls_connection
        let result = ensure_tls_connection(server).await;
        assert!(result.is_err(), "Should reject non-TLS connection");

        if let Err(e) = result {
            match e {
                ProxyError::NonTlsConnection(_) => {}, // Expected error
                _ => panic!("Expected NonTlsConnection error, got {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_ensure_tls_connection_with_no_data() {
        let (_, server) = create_tcp_pair().await;

        // Test ensure_tls_connection with no data
        let result = ensure_tls_connection(server).await;
        assert!(result.is_err(), "Should reject connection with no data");

        if let Err(e) = result {
            match e {
                ProxyError::NonTlsConnection(_) => {}, // Expected error
                _ => panic!("Expected NonTlsConnection error, got {:?}", e),
            }
        }
    }
}
