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

use crate::config::{self, ProxyConfig, ClientCertMode};

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
/// 檢查連接是否使用 TLS 協議
///
/// 通過查看前幾個字節來確定連接是否使用 TLS。
/// 如果不是 TLS 連接，則發送 TCP RST 立即關閉連接。
async fn ensure_tls_connection(stream: TcpStream) -> Result<TcpStream> {
    // 啟用 TCP_NODELAY 以獲得更快的響應
    stream.set_nodelay(true).map_err(ProxyError::Io)?;

    // 查看前幾個字節以檢查是否是 TLS ClientHello
    let mut peek_buf = [0u8; 5];

    // 使用超時避免無限等待
    match tokio::time::timeout(Duration::from_millis(500), stream.peek(&mut peek_buf)).await {
        // 成功查看數據
        Ok(Ok(size)) if size >= 3 => {
            // TLS 握手以內容類型 0x16 (22 十進制) 開始
            if peek_buf[0] != 0x16 {
                debug!("非 TLS 連接: 第一個字節是 {:#04x}, 預期 0x16", peek_buf[0]);
                send_tcp_rst(&stream)?;
                return Err(ProxyError::NonTlsConnection(format!("無效協議: 第一個字節 {:#04x}", peek_buf[0])));
            }

            debug!("檢測到 TLS 連接，繼續握手");
            Ok(stream)
        },
        // 數據不足以確定協議
        Ok(Ok(size)) => {
            debug!("數據不足 ({} 字節) 無法確定協議", size);
            send_tcp_rst(&stream)?;
            Err(ProxyError::NonTlsConnection(format!("數據不足: 僅收到 {} 字節", size)))
        },
        // 讀取套接字錯誤
        Ok(Err(e)) => {
            debug!("讀取套接字錯誤: {}", e);
            send_tcp_rst(&stream)?;
            Err(ProxyError::Io(e))
        },
        // 等待數據超時
        Err(_) => {
            debug!("等待初始數據超時");
            send_tcp_rst(&stream)?;
            Err(ProxyError::NonTlsConnection("等待初始數據超時".to_string()))
        }
    }
}

/// 發送 TCP RST 包立即關閉連接
fn send_tcp_rst(stream: &TcpStream) -> Result<()> {
    // 設置 SO_LINGER 為 0 會在關閉時發送 TCP RST
    stream.set_linger(Some(Duration::from_secs(0)))
        .map_err(|e| {
            debug!("設置 TCP RST 選項失敗: {}", e);
            ProxyError::Io(e)
        })
}

pub async fn handle_connection(
    client_stream: TcpStream,
    target_addr: SocketAddr,
    tls_acceptor: Arc<SslAcceptor>,
    config: &ProxyConfig,
) -> Result<()> {
    // 首先確保這是一個 TLS 連接
    let client_stream = ensure_tls_connection(client_stream).await?;

    // Setup TLS with client verification mode
    let mut ssl = openssl::ssl::Ssl::new(tls_acceptor.context()).map_err(ProxyError::Ssl)?;
    ssl.set_verify(match config.client_cert_mode {
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
        }
        return Err(ProxyError::TlsHandshake(e.to_string()));
    }

    debug!("TLS handshake successful");

    // Log TLS details and client certificate when appropriate
    if let (true, ssl) = (log::log_enabled!(log::Level::Debug), stream.as_ref().get_ref().ssl()) {
        debug!("TLS version: {}", ssl.version_str());
        debug!("TLS cipher: {}", ssl.current_cipher().map_or("None", |c| c.name()));
        debug!("TLS SNI: {}", ssl.servername(openssl::ssl::NameType::HOST_NAME).unwrap_or("None"));

        // Log client certificate if present and info logging is enabled
        if log::log_enabled!(log::Level::Info) {
            ssl.peer_certificate()
                .map(|cert| info!("Client certificate subject: {:?}", cert.subject_name()));
        }
    }

    // Connect to target with timeout
    let target_stream = timeout(
        Duration::from_secs(config::get_connection_timeout()),
        TcpStream::connect(target_addr)
    )
    .await
    .map_err(|_| ProxyError::ConnectionTimeout(config::get_connection_timeout()))?
    .map_err(ProxyError::Io)?;

    // Forward data between client and target
    proxy_data(stream, target_stream, config).await
}

#[cfg(test)]
mod tests {
    // Unit tests for connection handling could be added here
    // However, since we need to mock TLS connections, this might require more complex test setup
}
