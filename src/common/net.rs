//! 網絡相關工具函數
//!
//! 這個模組提供了網絡相關的工具函數。

use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;

use super::error::{ProxyError, Result};

/// 解析 socket 地址
///
/// # 參數
///
/// * `addr` - 要解析的地址字符串
///
/// # 返回
///
/// 返回解析後的 `SocketAddr`
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    // 嘗試直接解析
    if let Ok(socket_addr) = SocketAddr::from_str(addr) {
        return Ok(socket_addr);
    }
    
    // 嘗試使用 ToSocketAddrs 解析
    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err(ProxyError::Config(format!("無法解析地址: {}", addr)))
            }
        }
        Err(e) => Err(ProxyError::Config(format!("無法解析地址 {}: {}", addr, e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_socket_addr() {
        // 測試有效的地址
        let addr = parse_socket_addr("127.0.0.1:8080");
        assert!(addr.is_ok(), "應該能夠解析有效的地址");
        
        if let Ok(socket_addr) = addr {
            assert_eq!(socket_addr.port(), 8080);
        }
        
        // 測試無效的地址
        let addr = parse_socket_addr("invalid-address");
        assert!(addr.is_err(), "應該無法解析無效的地址");
    }
}
