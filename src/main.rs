//! Quantum Safe Proxy Command Line Tool
//!
//! This binary is the command-line interface for Quantum Safe Proxy.

use clap::Parser;
use log::{info, warn};

// Import our library
use quantum_safe_proxy::{Proxy, create_tls_acceptor, VERSION, APP_NAME, reload_config};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config;
use quantum_safe_proxy::config::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};
use quantum_safe_proxy::tls::{get_cert_subject, get_cert_fingerprint};
use quantum_safe_proxy::crypto::provider::environment::initialize_environment;

// Import for file and environment operations
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Quantum Safe Proxy: PQC-Enabled Sidecar with Hybrid Certificate Support
#[derive(Parser, Debug)]
#[clap(author, version = VERSION, about, long_about = None)]
struct Args {
    /// Listen to the address
    #[clap(short, long, default_value = LISTEN_STR)]
    listen: String,

    /// Target service address
    #[clap(short, long, default_value = TARGET_STR)]
    target: String,

    /// Server certificate path
    #[clap(long, default_value = CERT_PATH_STR)]
    cert: String,

    /// Server private key path
    #[clap(long, default_value = KEY_PATH_STR)]
    key: String,

    /// CA certificate path (for client certificate validation)
    #[clap(long, default_value = CA_CERT_PATH_STR)]
    ca_cert: String,

    /// Log level
    #[clap(long, default_value = LOG_LEVEL_STR)]
    log_level: String,

    /// Enable hybrid certificate mode
    #[clap(long)]
    hybrid_mode: bool,

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

    // Initialize environment
    // This ensures that environment checks are performed only once
    let env_info = initialize_environment();
    info!("Environment initialized: OpenSSL {}, PQC {}",
          &env_info.openssl_version,
          if env_info.pqc_available { "available" } else { "not available" });

    // Initialize configuration system
    let config = config::initialize(std::env::args().collect(), args.config_file.as_deref())?;

    // Log certificate information
    match get_cert_subject(&config.cert_path, None) {
        Ok(subject) => info!("Certificate subject: {}", subject),
        Err(e) => warn!("Unable to get certificate subject: {}", e),
    }
    match get_cert_fingerprint(&config.cert_path, None) {
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

    // Create proxy instance
    let proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
    );

    // Store proxy in shared state
    let proxy = Arc::new(Mutex::new(proxy));

    // Create configuration reload channel
    let (reload_tx, mut reload_rx) = mpsc::channel(1);
    let reload_tx_clone = reload_tx.clone();

    // Add configuration change listener
    config::add_listener(|event| {
        info!("Configuration change detected: {:?}", event);
    })?;

    let config_file = args.config_file.clone();

    // Set up configuration reload handler for Unix platforms
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let reload_tx = reload_tx_clone.clone();
        tokio::spawn(async move {
            let mut sig_hup = signal(SignalKind::hangup()).expect("Failed to create SIGHUP handler");

            info!("Configuration reload handler started (SIGHUP)");
            info!("To reload configuration, send SIGHUP signal to the process");

            while sig_hup.recv().await.is_some() {
                info!("SIGHUP received, triggering configuration reload");

                if reload_tx.send(()).await.is_err() {
                    warn!("Failed to send reload signal, channel closed");
                    break;
                }
            }
        });
    }

    // Set up configuration reload handler for non-Unix platforms
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

    // Spawn reload handler
    tokio::spawn(async move {
        while reload_rx.recv().await.is_some() {
            info!("Processing configuration reload request");

            // Get the config file path
            let config_path = if let Some(ref path) = config_file {
                path.clone()
            } else {
                // If no config file was specified, use default config file
                "config.json".to_string()
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

            match reload_config(&mut proxy_guard, Path::new(&config_path)) {
                Ok(_) => {
                    info!("Configuration updated successfully");
                },
                Err(e) => {
                    warn!("Failed to reload configuration: {}", e);
                }
            }
        }
    });

    // Get current configuration
    let current_config = config::get_config()?;

    // Start the proxy server
    info!("Listening on {}", current_config.listen);
    info!("Forwarding to {}", current_config.target);

    // Run the proxy
    proxy.lock().unwrap().run().await?;

    Ok(())
}
