//! Quantum Safe Proxy Command Line Interface
//!
//! This is the main entry point for the quantum-safe-proxy application.
//! It handles command line argument parsing, configuration loading, and
//! starting the proxy service.

use log::{info};
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};

use quantum_safe_proxy::{
    StandardProxyService, ProxyService,
    create_tls_acceptor
};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::{self};
use quantum_safe_proxy::crypto::initialize_openssl;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration with proper priority
    // This handles: defaults -> config file -> env vars -> CLI args
    let args = std::env::args().collect::<Vec<String>>();
    let initial_config = config::builder::auto_load(args)?;

    // 2. Initialize logger
    init_logger(initial_config.log_level());

    // 3. Initialize global configuration
    config::initialize(initial_config)?;
    info!("Configuration loaded successfully");

    // 4. Get the global configuration
    let config = config::get_config();

    // 5. Set OpenSSL directory if specified
    if let Some(openssl_dir) = config.openssl_dir() {
        info!("Setting OpenSSL directory to: {}", openssl_dir.display());
        std::env::set_var("OPENSSL_DIR", openssl_dir.to_string_lossy().to_string());
        initialize_openssl(openssl_dir);
    }

    // 6. Build certificate strategy and TLS acceptor
    let cert_strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)
        .and_then(|strategy| {
            strategy.downcast::<quantum_safe_proxy::tls::strategy::CertStrategy>()
                .map_err(|_| {
                    let err_msg = "Failed to downcast strategy to CertStrategy";
                    log::error!("{}", err_msg);
                    quantum_safe_proxy::common::ProxyError::Config(err_msg.to_string())
                })
                .map(|boxed| *boxed)
        })?;

    let tls_acceptor = create_tls_acceptor(
        config.client_ca_cert(),
        &config.client_cert_mode(),
        cert_strategy,
    )?;

    // 7. Start proxy service
    let listen_addr = config.listen();
    info!("Starting proxy service on {}", listen_addr);
    info!("Certificate mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });

    let proxy_service = StandardProxyService::new(
        listen_addr,
        config.target(),
        tls_acceptor,
        config.clone(),
    );
    let proxy_handle = proxy_service.start()?;

    // 8. Start admin server (if enabled via environment variable)
    let admin_api_enabled = std::env::var("ADMIN_API_ENABLED")
        .unwrap_or_else(|_| "0".to_string())
        .trim()
        .eq_ignore_ascii_case("1") ||
        std::env::var("ADMIN_API_ENABLED")
        .unwrap_or_else(|_| "0".to_string())
        .trim()
        .eq_ignore_ascii_case("true");

    let admin_server_handle = if admin_api_enabled {
        info!("Admin API is enabled");

        // Get admin server configuration from environment
        let admin_addr = std::env::var("ADMIN_API_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8443".to_string());

        let audit_log_path = std::env::var("ADMIN_AUDIT_LOG")
            .unwrap_or_else(|_| "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string());

        // Parse API keys from environment (format: "name:key:role,name:key:role")
        let api_keys = parse_api_keys_from_env();

        if api_keys.is_empty() {
            log::warn!("No API keys configured for admin server. Admin API will not accept any requests.");
        }

        let admin_config = quantum_safe_proxy::admin::server::AdminServerConfig {
            listen_addr: admin_addr.parse()
                .expect("Invalid ADMIN_API_ADDR format"),
            api_keys,
            audit_log_path,
        };

        // Spawn admin server in background task
        let handle = tokio::spawn(async move {
            if let Err(e) = quantum_safe_proxy::admin::start_admin_server(admin_config).await {
                log::error!("Admin server error: {}", e);
            }
        });

        Some(handle)
    } else {
        info!("Admin API is disabled (set ADMIN_API_ENABLED=1 to enable)");
        None
    };

    // 9. Wait for shutdown or reload signal
    let mut sighup = signal(SignalKind::hangup())?;
    tokio::spawn(async move {
        while let Some(_) = sighup.recv().await {
            info!("Received SIGHUP signal, reloading configuration...");
            // Configuration reload logic would go here
        }
    });

    // Wait for Ctrl+C
    signal::ctrl_c().await?;
    info!("Received shutdown signal");

    // Shutdown gracefully
    proxy_handle.shutdown().await?;

    // Shutdown admin server if running
    if let Some(handle) = admin_server_handle {
        handle.abort();
        info!("Admin server stopped");
    }

    info!("Proxy service stopped");

    Ok(())
}

/// Parse API keys from environment variable
fn parse_api_keys_from_env() -> Vec<quantum_safe_proxy::admin::types::ApiKey> {
    use quantum_safe_proxy::admin::types::{ApiKey, Role};

    let api_keys_str = match std::env::var("ADMIN_API_KEYS") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut api_keys = Vec::new();

    for entry in api_keys_str.split(',') {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() != 3 {
            log::warn!("Invalid API key entry format: {}", entry);
            continue;
        }

        let name = parts[0].trim().to_string();
        let key = parts[1].trim().to_string();
        let role_str = parts[2].trim().to_lowercase();

        let role = match role_str.as_str() {
            "admin" => Role::Admin,
            "operator" => Role::Operator,
            "viewer" => Role::Viewer,
            _ => {
                log::warn!("Invalid role '{}' for API key '{}', skipping", role_str, name);
                continue;
            }
        };

        api_keys.push(ApiKey {
            key,
            role,
            name,
            expires_at: None,
        });
    }

    api_keys
}
