//! Quantum Safe Proxy Command Line Interface
//!
//! This is the main entry point for the quantum-safe-proxy application.
//! It handles command line argument parsing, configuration loading, and
//! starting the proxy service.

use clap::{Parser, ArgAction};
use std::path::PathBuf;
use std::sync::Arc;

use quantum_safe_proxy::{
    StandardProxyService, ProxyService,
    create_tls_acceptor
};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::{ProxyConfig, ConfigLoader, ConfigMerger};
use quantum_safe_proxy::crypto::initialize_openssl;

/// Command line arguments for the quantum-safe-proxy
#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Quantum Safe Proxy with PQC and hybrid certificate support",
    long_about = "A TLS proxy with support for Post-Quantum Cryptography (PQC) and hybrid X.509 certificates. It can be deployed as a sidecar to provide PQC protection for existing services.",
    disable_version_flag = true
)]
struct CliArgs {
    /// Listen address (host:port)
    #[clap(long, value_name = "ADDR")]
    listen: Option<String>,

    /// Target address (host:port)
    #[clap(long, value_name = "ADDR")]
    target: Option<String>,

    /// Path to hybrid certificate file
    #[clap(long, value_name = "FILE")]
    hybrid_cert: Option<PathBuf>,

    /// Path to hybrid private key file
    #[clap(long, value_name = "FILE")]
    hybrid_key: Option<PathBuf>,

    /// Path to traditional certificate file
    #[clap(long, value_name = "FILE")]
    traditional_cert: Option<PathBuf>,

    /// Path to traditional private key file
    #[clap(long, value_name = "FILE")]
    traditional_key: Option<PathBuf>,

    /// Path to client CA certificate file
    #[clap(long, value_name = "FILE")]
    client_ca_cert: Option<PathBuf>,

    /// Log level (error, warn, info, debug, trace)
    #[clap(long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// Client certificate mode (required, optional, none)
    #[clap(long, value_name = "MODE")]
    client_cert_mode: Option<String>,

    /// Buffer size for data transfer
    #[clap(long, value_name = "SIZE")]
    buffer_size: Option<usize>,

    /// Connection timeout in seconds
    #[clap(long, value_name = "SECONDS")]
    connection_timeout: Option<u64>,

    /// OpenSSL directory
    #[clap(long, value_name = "DIR")]
    openssl_dir: Option<PathBuf>,

    /// Certificate strategy (single, sigalgs, dynamic)
    #[clap(long, value_name = "STRATEGY")]
    strategy: Option<String>,

    /// Path to PQC-only certificate file
    #[clap(long, value_name = "FILE")]
    pqc_only_cert: Option<PathBuf>,

    /// Path to PQC-only private key file
    #[clap(long, value_name = "FILE")]
    pqc_only_key: Option<PathBuf>,

    /// Path to configuration file
    #[clap(long, value_name = "FILE")]
    config_file: Option<PathBuf>,

    /// Print version information and exit
    #[clap(short = 'V', long, action = ArgAction::SetTrue)]
    show_version: bool,


}

/// Convert CLI arguments to a ProxyConfig
impl From<&CliArgs> for ProxyConfig {
    fn from(args: &CliArgs) -> Self {
        let mut config = ProxyConfig::default();

        // Only update fields that are explicitly specified in CLI args
        if let Some(listen) = &args.listen {
            if let Ok(addr) = listen.parse() {
                config.listen = addr;
            }
        }

        if let Some(target) = &args.target {
            if let Ok(addr) = target.parse() {
                config.target = addr;
            }
        }

        if let Some(hybrid_cert) = &args.hybrid_cert {
            config.hybrid_cert = hybrid_cert.clone();
        }

        if let Some(hybrid_key) = &args.hybrid_key {
            config.hybrid_key = hybrid_key.clone();
        }

        if let Some(traditional_cert) = &args.traditional_cert {
            config.traditional_cert = traditional_cert.clone();
        }

        if let Some(traditional_key) = &args.traditional_key {
            config.traditional_key = traditional_key.clone();
        }

        if let Some(client_ca_cert) = &args.client_ca_cert {
            config.client_ca_cert_path = client_ca_cert.clone();
        }

        if let Some(log_level) = &args.log_level {
            config.log_level = log_level.clone();
        }

        if let Some(client_cert_mode) = &args.client_cert_mode {
            if let Ok(mode) = client_cert_mode.parse() {
                config.client_cert_mode = mode;
            }
        }

        if let Some(buffer_size) = args.buffer_size {
            config.buffer_size = buffer_size;
        }

        if let Some(connection_timeout) = args.connection_timeout {
            config.connection_timeout = connection_timeout;
        }

        if let Some(openssl_dir) = &args.openssl_dir {
            config.openssl_dir = Some(openssl_dir.clone());
        }

        if let Some(strategy) = &args.strategy {
            if let Ok(s) = strategy.parse() {
                config.strategy = s;
            }
        }

        if let Some(pqc_only_cert) = &args.pqc_only_cert {
            config.pqc_only_cert = Some(pqc_only_cert.clone());
        }

        if let Some(pqc_only_key) = &args.pqc_only_key {
            config.pqc_only_key = Some(pqc_only_key.clone());
        }

        config
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli_args = CliArgs::parse();

    // Handle version flag
    if cli_args.show_version {
        println!("quantum-safe-proxy {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }



    // Load configuration with proper priority:
    // 1. Default values (lowest priority)
    // 2. Configuration file
    // 3. Environment variables
    // 4. Command line arguments (highest priority)
    let mut config = ProxyConfig::default();

    // Load from configuration file (if specified or default exists)
    let config_path = cli_args.config_file.as_deref().unwrap_or_else(|| {
        std::path::Path::new("config.json")
    });

    if config_path.exists() {
        if let Ok(file_config) = ProxyConfig::from_file(config_path) {
            config = config.merge(file_config);
        }
    }

    // Load from environment variables
    if let Ok(env_config) = ProxyConfig::from_env() {
        config = config.merge(env_config);
    }

    // Load from command line arguments (highest priority)
    let cli_config = ProxyConfig::from(&cli_args);
    config = config.merge(cli_config);

    // Initialize logger
    init_logger(&config.log_level);

    // Log the configuration
    use log::info;
    info!("Configuration loaded successfully");
    quantum_safe_proxy::config::log_config(&config);

    // Set OpenSSL directory if specified
    if let Some(openssl_dir) = &config.openssl_dir {
        std::env::set_var("OPENSSL_DIR", openssl_dir);
    }
    if let Some(openssl_dir) = config.openssl_dir.as_deref() {
        initialize_openssl(openssl_dir);
    }

    // Build certificate strategy and TLS acceptor
    let strategy = config.build_cert_strategy()?;
    let tls_acceptor = create_tls_acceptor(
        &config.client_ca_cert_path,
        &config.client_cert_mode,
        strategy,
    )?;

    // Start proxy service
    let proxy_service = StandardProxyService::new(
        config.listen,
        config.target,
        tls_acceptor,
        Arc::new(config),
    );
    let proxy_handle = proxy_service.start()?;

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    proxy_handle.shutdown().await?;

    Ok(())
}
