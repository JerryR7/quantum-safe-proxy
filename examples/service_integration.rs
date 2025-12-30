//! Service integration example
//!
//! This example demonstrates how to integrate Quantum Safe Proxy with other services.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use quantum_safe_proxy::config::parse_socket_addr;
use quantum_safe_proxy::tls::strategy::CertStrategy;
use std::path::Path;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Service Integration Example");
    println!("==========================");

    // Define addresses
    let backend_addr = parse_socket_addr("127.0.0.1:6000")?;
    let proxy_listen_addr = parse_socket_addr("0.0.0.0:8443")?;

    // Start the backend service in a separate task
    let backend_service = tokio::spawn(async move {
        if let Err(e) = run_backend_service(backend_addr).await {
            eprintln!("Backend service error: {}", e);
        }
    });

    // Give the backend service time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Backend service started at {}", backend_addr);

    // Create certificate strategy - Dynamic mode for auto-selection
    let strategy = CertStrategy::Dynamic {
        primary: (
            Path::new("certs/hybrid/dilithium3/server.crt").to_path_buf(),
            Path::new("certs/hybrid/dilithium3/server.key").to_path_buf(),
        ),
        fallback: (
            Path::new("certs/traditional/rsa/server.crt").to_path_buf(),
            Path::new("certs/traditional/rsa/server.key").to_path_buf(),
        ),
    };

    // Create TLS acceptor
    let tls_acceptor = create_tls_acceptor(
        Path::new("certs/hybrid/dilithium3/ca.crt"),
        &quantum_safe_proxy::config::ClientCertMode::Optional,
        strategy,
    )?;

    // Create proxy
    let config = std::sync::Arc::new(quantum_safe_proxy::config::ProxyConfig::default());

    let mut proxy = Proxy::new(
        proxy_listen_addr,
        backend_addr,
        tls_acceptor,
        config,
    );

    println!("Proxy service started at {}", proxy_listen_addr);
    println!("Forwarding traffic to backend at {}", backend_addr);
    println!("Press Ctrl+C to stop");

    // Run the proxy
    proxy.run().await?;

    let _ = backend_service.await;

    Ok(())
}

// Simple HTTP backend service
async fn run_backend_service(addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Backend service listening on {}", addr);

    loop {
        let (mut socket, client_addr) = listener.accept().await?;
        println!("Backend received connection from {}", client_addr);

        tokio::spawn(async move {
            let mut buffer = [0; 1024];

            match socket.read(&mut buffer).await {
                Ok(n) => {
                    let request = String::from_utf8_lossy(&buffer[..n]);
                    println!("Received request: {}", request.lines().next().unwrap_or(""));

                    let response = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/plain\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        "Hello from the backend service!\r\n",
                        "This connection was secured by Quantum Safe Proxy.\r\n"
                    );

                    if let Err(e) = socket.write_all(response.as_bytes()).await {
                        eprintln!("Failed to send response: {}", e);
                    }
                },
                Err(e) => eprintln!("Failed to read from socket: {}", e),
            }
        });
    }
}
