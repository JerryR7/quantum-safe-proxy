//! Quantum Safe Proxy Command Line Tool
//!
//! This binary is the command-line interface for Quantum Safe Proxy.

use clap::Parser;
use log::{info, warn};

// Import our library
use quantum_safe_proxy::{Proxy, create_tls_acceptor, VERSION, APP_NAME, reload_config};
use quantum_safe_proxy::common::{Result, init_logger, ProxyError, parse_socket_addr};
use quantum_safe_proxy::config::{ProxyConfig, ClientCertMode, ENV_PREFIX};
use quantum_safe_proxy::tls::{get_cert_subject, get_cert_fingerprint};

// Import for file and environment operations
use std::path::Path;
use std::fs;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

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
            match ProxyConfig::from_file(&config_file) {
                Ok(file_config) => {
                    config = config.merge(file_config);
                },
                Err(e) => {
                    warn!("Failed to load configuration file: {}", e);
                }
            }
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

    // Check configuration for warnings
    let warnings = config.check();
    for warning in warnings {
        warn!("{}", warning);
    }

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

    // Create proxy instance
    let proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
    );

    // Store configuration and proxy in shared state
    let config = Arc::new(Mutex::new(config));
    let proxy = Arc::new(Mutex::new(proxy));

    // Create a channel for reload signals
    let (reload_tx, mut reload_rx) = mpsc::channel::<()>(1);

    // Clone for signal handler
    let reload_tx_clone = reload_tx.clone();
    let config_file = args.config_file.clone();

    // Spawn signal handler for configuration reload
    #[cfg(unix)]
    {
        // Unix platforms: use SIGHUP signal
        let reload_tx = reload_tx_clone.clone();
        tokio::spawn(async move {
            // Create a signal handler for SIGHUP
            let mut sighup = match signal(SignalKind::hangup()) {
                Ok(signal) => signal,
                Err(e) => {
                    warn!("Failed to create SIGHUP handler: {}", e);
                    return;
                }
            };

            info!("Signal handler started, send SIGHUP to reload configuration");

            // Wait for SIGHUP signals
            while sighup.recv().await.is_some() {
                info!("Received SIGHUP signal, triggering configuration reload");
                if reload_tx.send(()).await.is_err() {
                    warn!("Failed to send reload signal, channel closed");
                    break;
                }
            }
        });
    }

    #[cfg(not(unix))]
    {
        // Windows and other platforms: use a timer-based approach
        let reload_tx = reload_tx_clone.clone();
        tokio::spawn(async move {
            info!("Configuration reload checker started (checking every 30 seconds)");
            info!("To reload configuration, modify the config file and wait for the next check");

            let mut last_modified = std::time::SystemTime::now();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Get the config file path
                let config_path = if let Some(ref path) = config_file.clone() {
                    path.clone()
                } else {
                    continue; // No config file specified
                };

                // Check if config file exists and has been modified
                let path = Path::new(&config_path);
                if !path.exists() {
                    continue;
                }

                // Check if file has been modified
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified {
                            info!("Configuration file modified, triggering reload");
                            last_modified = modified;

                            if reload_tx.send(()).await.is_err() {
                                warn!("Failed to send reload signal, channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    // Clone for reload handler
    let proxy_clone = proxy.clone();
    let config_clone = config.clone();

    // Spawn reload handler
    tokio::spawn(async move {
        while reload_rx.recv().await.is_some() {
            info!("Processing configuration reload request");

            // Get the config file path
            let config_path = if let Some(ref path) = config_file {
                path.clone()
            } else {
                // If no config file was specified, use environment-specific config
                let config_guard = config_clone.lock().unwrap();
                format!("config.{}.json", config_guard.environment)
            };

            // Check if config file exists
            if !Path::new(&config_path).exists() {
                warn!("Configuration file not found: {}", config_path);
                continue;
            }

            // Reload configuration
            let mut proxy_guard = match proxy_clone.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    warn!("Failed to acquire proxy lock: {}", e);
                    continue;
                }
            };

            let config_guard = match config_clone.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    warn!("Failed to acquire config lock: {}", e);
                    continue;
                }
            };

            match reload_config(&mut proxy_guard, &config_guard, Path::new(&config_path)) {
                Ok(new_config) => {
                    // Update the stored configuration
                    drop(config_guard); // Release the lock before acquiring it again
                    if let Ok(mut config_guard) = config_clone.lock() {
                        *config_guard = new_config;
                        info!("Configuration updated successfully");
                    }
                },
                Err(e) => {
                    warn!("Failed to reload configuration: {}", e);
                }
            }
        }
    });

    #[cfg(unix)]
    info!("Proxy service ready, press Ctrl+C to stop (send SIGHUP to reload configuration)");

    #[cfg(not(unix))]
    info!("Proxy service ready, press Ctrl+C to stop (configuration will be reloaded automatically when the file is modified)");

    // Run proxy service
    let proxy_guard = proxy.lock().unwrap();
    proxy_guard.run().await?;

    Ok(())
}
