//! Protocol detector implementation
//!
//! This module implements protocol detection by examining the first few bytes
//! of a connection. It is designed to be efficient and non-blocking, similar
//! to how NGINX and HAProxy implement protocol detection.

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use log::{debug, trace};

use crate::common::{ProxyError, Result};

/// Protocol detection result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionResult {
    /// TLS protocol detected
    Tls,
    /// Non-TLS protocol detected
    NonTls(String),
    /// Need more data to determine protocol
    NeedMoreData,
}

/// Protocol information
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    /// Protocol name
    pub name: String,
    /// Protocol version (if available)
    pub version: Option<String>,
}

/// Protocol detector trait
///
/// This trait defines the interface for protocol detection.
pub trait ProtocolDetector: Send + Sync {
    /// Detect protocol from a TCP stream
    ///
    /// This method examines the first few bytes of a TCP stream to determine
    /// the protocol being used. It is designed to be non-blocking and efficient.
    ///
    /// # Parameters
    ///
    /// * `stream` - TCP stream to examine
    /// * `timeout_ms` - Timeout in milliseconds for reading data
    ///
    /// # Returns
    ///
    /// Returns a result containing the detection result
    #[allow(async_fn_in_trait)]
    async fn detect(&self, stream: &mut TcpStream, timeout_ms: u64) -> Result<DetectionResult>;

    /// Get protocol information
    ///
    /// This method returns detailed information about the detected protocol.
    /// It should be called after `detect` returns `DetectionResult::Tls`.
    ///
    /// # Parameters
    ///
    /// * `data` - Protocol data (e.g., TLS ClientHello)
    ///
    /// # Returns
    ///
    /// Returns protocol information
    fn get_protocol_info(&self, data: &[u8]) -> Option<ProtocolInfo>;
}

/// TLS protocol detector
///
/// This detector examines the first few bytes of a connection to determine
/// if it is using the TLS protocol. It looks for the TLS handshake record
/// type (0x16) and version information.
#[derive(Debug, Clone)]
pub struct TlsDetector {
    /// Minimum bytes required for detection
    min_bytes: usize,
    /// Maximum bytes to read for detection
    max_bytes: usize,
}

impl Default for TlsDetector {
    fn default() -> Self {
        Self {
            min_bytes: 5,  // Minimum bytes needed to identify TLS
            max_bytes: 16, // Read a bit more for better detection
        }
    }
}

impl TlsDetector {
    /// Create a new TLS detector with custom parameters
    ///
    /// # Parameters
    ///
    /// * `min_bytes` - Minimum bytes required for detection
    /// * `max_bytes` - Maximum bytes to read for detection
    ///
    /// # Returns
    ///
    /// Returns a new TLS detector
    pub fn new(min_bytes: usize, max_bytes: usize) -> Self {
        Self {
            min_bytes,
            max_bytes,
        }
    }

    /// Check if data appears to be a TLS ClientHello
    ///
    /// # Parameters
    ///
    /// * `data` - Data to examine
    ///
    /// # Returns
    ///
    /// Returns the detection result
    fn check_protocol(&self, data: &[u8]) -> DetectionResult {
        // Not enough data to determine protocol
        if data.len() < self.min_bytes {
            trace!("Not enough data to determine protocol: got {} bytes, need {}", data.len(), self.min_bytes);
            return DetectionResult::NeedMoreData;
        }

        // TLS handshake record type is 0x16 (22)
        if data[0] != 0x16 {
            let reason = format!("Non-TLS protocol detected: first byte is {:#04x}, expected 0x16", data[0]);
            debug!("{}", reason);
            return DetectionResult::NonTls(reason);
        }

        // Check TLS version (major.minor)
        let major = data[1];
        let minor = data[2];

        // Valid TLS versions
        let valid_version = (major == 0x03 && (minor >= 0x01 && minor <= 0x04)) ||
                           // SSLv3
                           (major == 0x03 && minor == 0x00);

        if !valid_version {
            trace!("Invalid TLS version: {}.{}", major, minor);
            return DetectionResult::NonTls(format!("Invalid TLS version: {}.{}", major, minor));
        }

        // Check record length
        let record_length = ((data[3] as usize) << 8) | (data[4] as usize);
        if record_length < 4 || record_length > 16384 {
            trace!("Invalid TLS record length: {}", record_length);
            return DetectionResult::NonTls(format!("Invalid TLS record length: {}", record_length));
        }

        debug!("TLS protocol detected");
        DetectionResult::Tls
    }

    /// Extract TLS version from ClientHello
    ///
    /// # Parameters
    ///
    /// * `data` - TLS ClientHello data
    ///
    /// # Returns
    ///
    /// Returns the TLS version as a string
    fn extract_tls_version(&self, data: &[u8]) -> Option<String> {
        if data.len() < 3 {
            return None;
        }

        let major = data[1];
        let minor = data[2];

        match (major, minor) {
            (0x03, 0x00) => Some("SSLv3".to_string()),
            (0x03, 0x01) => Some("TLSv1.0".to_string()),
            (0x03, 0x02) => Some("TLSv1.1".to_string()),
            (0x03, 0x03) => Some("TLSv1.2".to_string()),
            (0x03, 0x04) => Some("TLSv1.3".to_string()),
            _ => Some(format!("Unknown ({}.{})", major, minor)),
        }
    }
}

impl ProtocolDetector for TlsDetector {
    async fn detect(&self, stream: &mut TcpStream, timeout_ms: u64) -> Result<DetectionResult> {
        // Create buffer for peeking data
        let mut peek_buf = vec![0u8; self.max_bytes];

        // Use timeout to avoid infinite waiting
        match timeout(Duration::from_millis(timeout_ms), stream.peek(&mut peek_buf)).await {
            // Successfully peeked data
            Ok(Ok(size)) if size >= self.min_bytes => {
                trace!("Peeked {} bytes: {:02X?}", size, &peek_buf[..size]);

                let result = self.check_protocol(&peek_buf[..size]);
                Ok(result)
            },
            // Not enough data
            Ok(Ok(size)) => {
                trace!("Not enough data to determine protocol: got {} bytes, need {}", size, self.min_bytes);
                Ok(DetectionResult::NeedMoreData)
            },
            // Error peeking data
            Ok(Err(e)) => {
                debug!("Error peeking data: {}", e);
                Err(ProxyError::Io(e))
            },
            // Timeout waiting for data
            Err(_) => {
                debug!("Timeout waiting for protocol data");
                Ok(DetectionResult::NeedMoreData)
            }
        }
    }

    fn get_protocol_info(&self, data: &[u8]) -> Option<ProtocolInfo> {
        if data.len() < self.min_bytes || data[0] != 0x16 {
            return None;
        }

        let version = self.extract_tls_version(data);

        Some(ProtocolInfo {
            name: "TLS".to_string(),
            version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::io::AsyncWriteExt;

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
    async fn test_tls_detection() {
        let (mut client, mut server) = create_tcp_pair().await;

        // Simulate TLS ClientHello
        let tls_client_hello = [
            0x16, 0x03, 0x03, 0x00, 0x31, // TLS record header (type, version, length)
            0x01, 0x00, 0x00, 0x2d, 0x03, 0x03, // Handshake header
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Random (truncated)
        ];

        // Send TLS ClientHello from client to server
        client.write_all(&tls_client_hello).await.unwrap();

        // Create detector and detect protocol
        let detector = TlsDetector::default();
        let result = detector.detect(&mut server, 100).await.unwrap();

        assert_eq!(result, DetectionResult::Tls);
    }

    #[tokio::test]
    async fn test_non_tls_detection() {
        let (mut client, mut server) = create_tcp_pair().await;

        // Simulate HTTP request
        let http_request = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";

        // Send HTTP request from client to server
        client.write_all(http_request).await.unwrap();

        // Create detector and detect protocol
        let detector = TlsDetector::default();
        let result = detector.detect(&mut server, 100).await.unwrap();

        match result {
            DetectionResult::NonTls(_) => assert!(true),
            _ => panic!("Expected NonTls, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_need_more_data() {
        let (_, mut server) = create_tcp_pair().await;

        // Create detector and detect protocol
        let detector = TlsDetector::default();

        // Since we didn't send any data, we should get NeedMoreData
        // We'll use a timeout to avoid hanging the test
        let result = detector.detect(&mut server, 10).await.unwrap();

        // We should get NeedMoreData
        assert_eq!(result, DetectionResult::NeedMoreData);
    }
}
