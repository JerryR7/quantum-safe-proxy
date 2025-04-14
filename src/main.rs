//! Quantum Safe Proxy Command Line Tool
//!
//! This binary is the command-line interface for Quantum Safe Proxy.

use clap::Parser;
use log::{info, warn};

// Import our library
use quantum_safe_proxy::{Proxy, create_tls_acceptor, VERSION, APP_NAME};
use quantum_safe_proxy::common::{Result, init_logger, ProxyError, parse_socket_addr};
use quantum_safe_proxy::config::{ProxyConfig, ClientCertMode, ENV_PREFIX};
use quantum_safe_proxy::tls::{get_cert_subject, get_cert_fingerprint};

// Import for file and environment operations
use std::path::Path;
use std::fs;
use std::env;

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
    #[clap(long, default_value = "certs/hybrid/dilithium3/server.crt")]
    cert: String,

    /// Server private key path
    #[clap(long, default_value = "certs/hybrid/dilithium3/server.key")]
    key: String,

    /// CA certificate path (for client certificate validation)
    #[clap(long, default_value = "certs/hybrid/dilithium3/ca.crt")]
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

    /// Environment (development, testing, production)
    #[clap(long, default_value = "production")]
    environment: String,

    /// Client certificate verification mode (required, optional, none)
    /// - required: Client must provide a valid certificate
    /// - optional: Client certificate is verified if provided
    /// - none: No client certificate verification
    #[clap(long, default_value = "optional")]
    client_cert_mode: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logger
    init_logger(&args.log_level);

    info!("Starting {} v{}", APP_NAME, VERSION);

    // Create default configuration
    let mut config = ProxyConfig::default();

    // Load environment-specific configuration if it exists
    let env_config_path = format!("config.{}.json", args.environment);
    if Path::new(&env_config_path).exists() {
        info!("Loading environment-specific configuration from {}", env_config_path);
        let env_config_str = fs::read_to_string(&env_config_path)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to read environment configuration file: {}", e
            )))?;

        let env_config: ProxyConfig = serde_json::from_str(&env_config_str)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to parse environment configuration file: {}", e
            )))?;

        config = config.merge(env_config);
    }

    // Load from configuration file if specified
    if let Some(config_file) = args.config_file.clone() {
        if Path::new(&config_file).exists() {
            info!("Loading configuration from file: {}", config_file);
            let config_str = fs::read_to_string(&config_file)
                .map_err(|e| ProxyError::Config(format!(
                    "Failed to read configuration file: {}", e
                )))?;

            let file_config: ProxyConfig = serde_json::from_str(&config_str)
                .map_err(|e| ProxyError::Config(format!(
                    "Failed to parse configuration file: {}", e
                )))?;

            config = config.merge(file_config);
        } else {
            warn!("Configuration file not found: {}", config_file);
        }
    }

    // Load from environment variables if specified
    if args.from_env {
        info!("Loading configuration from environment variables");
        // Helper function to get environment variable with prefix
        let get_env = |name: &str| -> Option<String> {
            let full_name = format!("{}{}", ENV_PREFIX, name);
            env::var(&full_name).ok()
        };

        // Load each configuration option from environment variables
        let mut env_config = ProxyConfig::default();

        if let Some(listen) = get_env("LISTEN") {
            env_config.listen = parse_socket_addr(&listen)?;
        }

        if let Some(target) = get_env("TARGET") {
            env_config.target = parse_socket_addr(&target)?;
        }

        if let Some(cert) = get_env("CERT") {
            env_config.cert_path = cert.into();
        }

        if let Some(key) = get_env("KEY") {
            env_config.key_path = key.into();
        }

        if let Some(ca_cert) = get_env("CA_CERT") {
            env_config.ca_cert_path = ca_cert.into();
        }

        if let Some(hybrid_mode) = get_env("HYBRID_MODE") {
            env_config.hybrid_mode = hybrid_mode.to_lowercase() == "true";
        }

        if let Some(log_level) = get_env("LOG_LEVEL") {
            env_config.log_level = log_level;
        }

        if let Some(client_cert_mode) = get_env("CLIENT_CERT_MODE") {
            env_config.client_cert_mode = ClientCertMode::from_str(&client_cert_mode)?;
        }

        if let Some(env_name) = get_env("ENVIRONMENT") {
            env_config.environment = env_name;
        }

        config = config.merge(env_config);
    } else {
        // Load from command line arguments
        info!("Loading configuration from command line arguments");
        let cmd_config = ProxyConfig::from_args(
            &args.listen,
            &args.target,
            &args.cert,
            &args.key,
            &args.ca_cert,
            &args.log_level,
            &args.client_cert_mode,
        )?;

        // Set hybrid mode from command line
        if args.hybrid_mode {
            let mut hybrid_config = cmd_config.clone();
            hybrid_config.hybrid_mode = true;
            config = config.merge(hybrid_config);
        } else {
            config = config.merge(cmd_config);
        }
    }

    // Validate the final configuration
    config.validate()?;

    info!("Configuration loaded successfully");

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
        &config.client_cert_mode,
    )?;

    // Log client certificate mode
    info!("Client certificate mode: {}", config.client_cert_mode);

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
