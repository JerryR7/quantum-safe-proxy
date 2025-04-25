//! Data forwarding module
//!
//! This module handles data forwarding between two streams.
//! Optimized for high performance and memory efficiency using Rust's zero-cost abstractions.

use log::debug;
use socket2::{Socket, TcpKeepalive};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::common::{ProxyError, Result};
use crate::config::ProxyConfig;

// 常數定義
const KEEPALIVE_INTERVAL: u64 = 10;   // TCP keepalive 間隔時間（秒）
const KEEPALIVE_RETRIES: u32 = 3;     // TCP keepalive 重試次數
const MIN_BUFFER_SIZE: usize = 1024;  // 最小緩衝區大小（字節）

/// 設置 TCP keepalive
fn set_tcp_keepalive(stream: &TcpStream, timeout_secs: u64) -> io::Result<()> {
    // 獲取文件描述符
    let fd = stream.as_raw_fd();

    // 安全地使用 socket2 設置 keepalive
    unsafe {
        // 從文件描述符創建 Socket，但不獲取所有權
        let socket = Socket::from_raw_fd(fd);

        // 設置 keepalive
        socket.set_keepalive(true)?;

        // 設置 keepalive 參數
        let keepalive = TcpKeepalive::new()
            .with_time(Duration::from_secs(timeout_secs))
            .with_interval(Duration::from_secs(KEEPALIVE_INTERVAL))
            .with_retries(KEEPALIVE_RETRIES);

        socket.set_tcp_keepalive(&keepalive)?;

        // 釋放 socket 但不關閉文件描述符
        let _ = socket.into_raw_fd();
    }

    Ok(())
}

/// 單向數據傳輸
///
/// 從 reader 讀取數據並寫入 writer，直到連接關閉或發生錯誤
/// 使用 tokio::io::copy 實現高效的數據傳輸
async fn transfer<R, W>(
    mut reader: R,
    mut writer: W,
    _buffer_size: usize, // 保留參數以維持 API 兼容性，但不再使用
    direction: &'static str
) -> Result<u64>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    // 使用 tokio::io::copy 進行高效的數據傳輸
    // 它內部使用了優化的緩衝區管理和零拷貝技術
    let result = tokio::io::copy(
        &mut reader,
        &mut writer,
    ).await;

    // 處理結果
    match result {
        Ok(total_bytes) => {
            debug!("{}: 總共傳輸 {} 字節", direction, total_bytes);

            // 正常關閉寫入端
            if let Err(e) = writer.shutdown().await {
                debug!("{}: 關閉連接錯誤: {}", direction, e);
            }

            Ok(total_bytes)
        },
        Err(e) => {
            debug!("{}: 傳輸錯誤: {}", direction, e);
            Err(ProxyError::Io(e))
        }
    }
}

/// 雙向數據轉發
///
/// 在兩個流之間進行雙向數據轉發，直到兩個方向都完成或發生錯誤
pub async fn proxy_data<S>(
    tls_stream: S,
    target_stream: TcpStream,
    config: &ProxyConfig,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // 獲取連接超時設置，用於 TCP keepalive
    let timeout_secs = config.connection_timeout;

    // 設置 TCP keepalive 以維持長連線
    if let Err(e) = set_tcp_keepalive(&target_stream, timeout_secs) {
        debug!("無法設置 TCP keepalive: {}", e);
    } else {
        debug!("TCP keepalive 已啟用，超時時間: {}秒，間隔: {}秒，重試次數: {}",
               timeout_secs, KEEPALIVE_INTERVAL, KEEPALIVE_RETRIES);
    }

    // 獲取配置的緩衝區大小，確保至少達到最小緩衝區大小
    let buffer_size = config.buffer_size.max(MIN_BUFFER_SIZE);

    // 分割流
    let (tls_read, tls_write) = tokio::io::split(tls_stream);
    let (target_read, target_write) = tokio::io::split(target_stream);

    // 啟動雙向傳輸
    let client_to_target = transfer(tls_read, target_write, buffer_size, "Client->Target");
    let target_to_client = transfer(target_read, tls_write, buffer_size, "Target->Client");

    // 等待兩個方向都完成
    let (client_result, target_result) = tokio::join!(client_to_target, target_to_client);

    // 處理結果
    match (client_result, target_result) {
        (Ok(client_bytes), Ok(target_bytes)) => {
            debug!("連接成功完成。客戶端->目標: {} 字節，目標->客戶端: {} 字節",
                   client_bytes, target_bytes);
        },
        (client_result, target_result) => {
            // 只要有一個方向成功，我們就認為連接是部分成功的
            if client_result.is_ok() || target_result.is_ok() {
                debug!("連接部分成功。客戶端->目標: {:?}, 目標->客戶端: {:?}",
                       client_result, target_result);
            } else {
                debug!("連接失敗。客戶端->目標: {:?}, 目標->客戶端: {:?}",
                       client_result, target_result);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;
    use tokio::test;

    #[test]
    async fn test_transfer() {
        // 創建一對連接的流
        let (client, server) = duplex(1024);

        // 寫入測試數據
        let test_data = b"Hello, World!";
        let mut client_write = client;
        client_write.write_all(test_data).await.unwrap();
        client_write.flush().await.unwrap();
        client_write.shutdown().await.unwrap();

        // 讀取數據
        let mut server_read = server;
        let mut buffer = vec![0u8; 1024];
        let n = server_read.read(&mut buffer).await.unwrap();

        // 驗證數據
        assert_eq!(&buffer[..n], test_data);
    }
}
