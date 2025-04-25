//! Quantum Safe Proxy Command Line Interface

use clap::Parser;
use log::{info, warn};


use quantum_safe_proxy::{Proxy, create_tls_acceptor, VERSION, APP_NAME, reload_config};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config;
use quantum_safe_proxy::config::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};
use quantum_safe_proxy::tls::{get_cert_subject, get_cert_fingerprint};
use quantum_safe_proxy::crypto::{check_environment, initialize_openssl};


use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(author, version = VERSION, about, long_about = None)]
struct Args {
    /// Listen to the address
    #[clap(short, long, default_value = LISTEN_STR)]
    listen: String,

    /// Target service address
    #[clap(short, long, default_value = TARGET_STR)]
    target: String,

    /// Server certificate path (legacy parameter, use classic-cert instead)
    #[clap(long, default_value = CERT_PATH_STR)]
    cert: String,

    /// Server private key path (legacy parameter, use classic-key instead)
    #[clap(long, default_value = KEY_PATH_STR)]
    key: String,

    /// Path to classic (RSA/ECDSA) cert PEM
    #[clap(long)]
    classic_cert: Option<String>,

    /// Path to classic private key PEM
    #[clap(long)]
    classic_key: Option<String>,

    /// Always use SigAlgs strategy: auto-select cert by client signature_algorithms
    #[clap(long)]
    use_sigalgs: Option<bool>,

    /// CA certificate path (for client certificate validation)
    #[clap(long, default_value = CA_CERT_PATH_STR)]
    ca_cert: String,

    /// Log level
    #[clap(long, default_value = LOG_LEVEL_STR)]
    log_level: String,

    /// Load configuration from a file
    #[clap(long)]
    config_file: Option<String>,

    /// Client certificate verification mode (required, optional, none)
    /// - required: Client must provide a valid certificate
    /// - optional: Client certificate is verified if provided
    /// - none: No client certificate verification
    #[clap(long, default_value = "optional")]
    client_cert_mode: String,

    /// Buffer size for data transfer in bytes (default: 8192)
    #[clap(long, default_value = "8192")]
    buffer_size: usize,

    /// Connection timeout in seconds (default: 30)
    #[clap(long, default_value = "30")]
    connection_timeout: u64,

    /// OpenSSL installation directory
    /// If specified, this will be used to locate OpenSSL libraries and headers
    #[clap(long)]
    openssl_dir: Option<PathBuf>,

    // environment 參數已移除，不再支持環境特定配置文件
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logger
    // 可以使用以下環境變數控制日誌級別:
    // 1. QUANTUM_SAFE_PROXY_LOG_LEVEL=debug (優先)
    // 2. RUST_LOG=quantum_safe_proxy=debug (其次)
    // 3. 命令行參數 --log-level debug (最後)
    //
    // 例如: RUST_LOG=quantum_safe_proxy::config=debug,quantum_safe_proxy=warn
    init_logger(&args.log_level);

    info!("Starting {} v{}", APP_NAME, VERSION);

    // Set OpenSSL directory if specified in command line arguments
    if let Some(openssl_dir) = &args.openssl_dir {
        info!("Setting OpenSSL directory to: {}", openssl_dir.display());
        std::env::set_var("OPENSSL_DIR", openssl_dir.to_string_lossy().to_string());
    }

    // Initialize configuration system first
    let mut config = config::initialize(std::env::args().collect(), args.config_file.as_deref())?;

    // Only update certificate paths from command line if they are explicitly specified
    let default_cert = PathBuf::from(CERT_PATH_STR);
    let default_key = PathBuf::from(KEY_PATH_STR);

    // For classic_cert and classic_key, only override if explicitly specified
    if let Some(classic_cert) = &args.classic_cert {
        config.classic_cert = PathBuf::from(classic_cert);
    }

    if let Some(classic_key) = &args.classic_key {
        config.classic_key = PathBuf::from(classic_key);
    }

    // Only override use_sigalgs if explicitly specified in command line
    if let Some(use_sigalgs) = args.use_sigalgs {
        config.use_sigalgs = use_sigalgs;
    }

    // For backward compatibility, also update the legacy cert_path and key_path
    // but only if they are different from the defaults
    if PathBuf::from(&args.cert) != default_cert {
        config.cert_path = PathBuf::from(&args.cert);
    }

    if PathBuf::from(&args.key) != default_key {
        config.key_path = PathBuf::from(&args.key);
    }

    // Configuration details are already logged in config module

    // Initialize OpenSSL from the specified directory

    // Try command line argument first
    if let Some(openssl_dir) = &args.openssl_dir {
        let success = initialize_openssl(openssl_dir);
        if !success {
            warn!("Failed to initialize OpenSSL from command line directory: {}", openssl_dir.display());
        }
    }
    // Otherwise, try configuration
    else if let Some(openssl_dir) = &config.openssl_dir {
        let success = initialize_openssl(openssl_dir);
        if !success {
            warn!("Failed to initialize OpenSSL from configured directory: {}", openssl_dir.display());
        }
    }

    // Now check environment after initializing OpenSSL
    let env_info = check_environment();
    info!("Environment initialized: OpenSSL {}, PQC {}",
          &env_info.openssl_version,
          if env_info.pqc_available { "available" } else { "not available" });

    // Log certificate information
    match get_cert_subject(&config.cert_path) {
        Ok(subject) => info!("Certificate subject: {}", subject),
        Err(e) => warn!("Unable to get certificate subject: {}", e),
    }
    match get_cert_fingerprint(&config.cert_path) {
        Ok(fingerprint) => info!("Certificate fingerprint: {}", fingerprint),
        Err(e) => warn!("Unable to get certificate fingerprint: {}", e),
    }

    // Build certificate strategy
    let strategy = config.build_cert_strategy()?;

    // Create TLS acceptor with the certificate strategy
    let tls_acceptor = create_tls_acceptor(
        &config.ca_cert_path,
        &config.client_cert_mode,
        strategy,
    )?;

    // Create proxy instance
    let proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
        Arc::new(config),  // 將 ProxyConfig 包裝在 Arc 中
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
