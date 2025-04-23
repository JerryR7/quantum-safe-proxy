//! Network utility functions
//!
//! This module provides utility functions for network operations.

use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;

use super::error::{ProxyError, Result};

/// Parse a socket address
///
/// # Arguments
///
/// * `addr` - The address string to parse
///
/// # Returns
///
/// The parsed `SocketAddr`
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    // Try direct parsing first
    if let Ok(socket_addr) = SocketAddr::from_str(addr) {
        return Ok(socket_addr);
    }

    // Try using ToSocketAddrs trait
    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err(ProxyError::Config(format!("Failed to parse address: {}", addr)))
            }
        }
        Err(e) => Err(ProxyError::Config(format!("Failed to parse address {}: {}", addr, e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_socket_addr() {
        // Test valid address
        let addr = parse_socket_addr("127.0.0.1:8080");
        assert!(addr.is_ok(), "Should be able to parse a valid address");

        if let Ok(socket_addr) = addr {
            assert_eq!(socket_addr.port(), 8080);
        }

        // Test invalid address
        let addr = parse_socket_addr("invalid-address");
        assert!(addr.is_err(), "Should fail to parse an invalid address");
    }
}
