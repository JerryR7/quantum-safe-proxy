//! 簡單代理示例
//!
//! 這個示例展示了如何創建一個簡單的 Quantum Safe Proxy 實例。

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result, parse_socket_addr};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    env_logger::init();

    println!("啟動簡單代理示例...");

    // 創建 TLS 接受器
    let tls_acceptor = create_tls_acceptor(
        Path::new("certs/server.crt"),
        Path::new("certs/server.key"),
        Path::new("certs/ca.crt"),
    )?;

    // 創建並啟動代理
    let listen_addr = parse_socket_addr("0.0.0.0:8443")?;
    let target_addr = parse_socket_addr("127.0.0.1:6000")?;

    let proxy = Proxy::new(
        listen_addr,
        target_addr,
        tls_acceptor,
    );

    println!("代理服務已啟動，按 Ctrl+C 停止");

    // 運行代理服務
    proxy.run().await?;

    Ok(())
}
