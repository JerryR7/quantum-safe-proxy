//! Quantum Safe Proxy Command Line Tool
//!
//! This binary is the command-line interface for Quantum Safe Proxy.

use clap::Parser;
use log::{info, warn};

// Import our library
use quantum_safe_proxy::{Proxy, create_tls_acceptor, VERSION, APP_NAME};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::ProxyConfig;
use quantum_safe_proxy::tls::{get_cert_subject, get_cert_fingerprint};

/// Quantum Safe Proxy: PQC-Enabled Sidecar with Hybrid Certificate Support
#[derive(Parser, Debug)]
#[clap(author, version = VERSION, about, long_about = None)]
struct Args {
    /// Listen address
    #[clap(short, long, default_value = "0.0.0.0:8443")]
    listen: String,

    /// Target service address
    #[clap(short, long, default_value = "127.0.0.1:6000")]
    target: String,

    /// Server certificate path
    #[clap(long, default_value = "certs/server.crt")]
    cert: String,

    /// Server private key path
    #[clap(long, default_value = "certs/server.key")]
    key: String,

    /// CA certificate path (for client certificate validation)
    #[clap(long, default_value = "certs/ca.crt")]
    ca_cert: String,

    /// Log level
    #[clap(long, default_value = "info")]
    log_level: String,

    /// Enable hybrid certificate mode
    #[clap(long)]
    hybrid_mode: bool,

    /// Load configuration from environment variables
    #[clap(long)]
    from_env: bool,

    /// Load configuration from a file
    #[clap(long)]
    config_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logger
    init_logger(&args.log_level);

    info!("Starting {} v{}", APP_NAME, VERSION);

    // Choose configuration source based on arguments
    let config = if args.from_env {
        info!("Loading configuration from environment variables");
        ProxyConfig::from_env()?
    } else if let Some(config_file) = args.config_file.clone() {
        info!("Loading configuration from file: {}", config_file);
        ProxyConfig::from_file(&config_file)?
    } else {
        info!("Loading configuration from command line arguments");
        ProxyConfig::from_args(
            &args.listen,
            &args.target,
            &args.cert,
            &args.key,
            &args.ca_cert,
            &args.log_level,
        )?
    };

    // Validate configuration
    config.validate()?;

    info!("Listen address: {}", config.listen);
    info!("Target service: {}", config.target);
    info!("Using certificate: {:?}", config.cert_path);

    // Try to get certificate subject
    match get_cert_subject(&config.cert_path) {
        Ok(subject) => info!("Certificate subject: {}", subject),
        Err(e) => warn!("Unable to get certificate subject: {}", e),
    }

    // Try to get certificate fingerprint
    match get_cert_fingerprint(&config.cert_path) {
        Ok(fingerprint) => info!("Certificate fingerprint: {}", fingerprint),
        Err(e) => warn!("Unable to get certificate fingerprint: {}", e),
    }

    // Create TLS acceptor
    let tls_acceptor = create_tls_acceptor(
        &config.cert_path,
        &config.key_path,
        &config.ca_cert_path,
    )?;

    // Create and start proxy
    let proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
    );

    info!("Proxy service ready, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    Ok(())
}
