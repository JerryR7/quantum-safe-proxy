//! Logging utility functions
//!
//! This module provides utility functions for the logging system.

/// Initialize the logging system
///
/// # Arguments
///
/// * `level` - Log level
pub fn init_logger(level: &str) {
    // First check QUANTUM_SAFE_PROXY_LOG_LEVEL environment variable
    let log_level = std::env::var("QUANTUM_SAFE_PROXY_LOG_LEVEL").unwrap_or_else(|_| level.to_string());

    // If log_level doesn't contain module name, add default module name
    let log_level = if !log_level.contains('=') && !log_level.is_empty() {
        format!("quantum_safe_proxy={}", log_level)
    } else {
        log_level
    };

    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", &log_level);

    // Try to initialize the logger, but don't panic if it's already initialized
    if let Err(e) = env_logger::try_init_from_env(env) {
        eprintln!("Failed to initialize logger: {}", e);
    } else {
        log::debug!("Logger initialized with level: {}", log_level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logger() {
        // Test logger initialization
        // Note: This test might affect other tests since it initializes the global logger
        // So we just ensure the function doesn't crash
        init_logger("debug");
    }
}
