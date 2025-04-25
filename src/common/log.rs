//! Logging utility functions
//!
//! This module provides utility functions for the logging system.

/// Initialize the logging system
///
/// # Arguments
///
/// * `level` - Log level
pub fn init_logger(level: &str) {
    // 首先檢查 QUANTUM_SAFE_PROXY_LOG_LEVEL 環境變數
    let log_level = std::env::var("QUANTUM_SAFE_PROXY_LOG_LEVEL").unwrap_or_else(|_| level.to_string());

    // 如果 log_level 不包含模組名稱，則添加默認的模組名稱
    let log_level = if !log_level.contains('=') && !log_level.is_empty() {
        format!("quantum_safe_proxy={}", log_level)
    } else {
        log_level
    };

    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", &log_level);

    env_logger::init_from_env(env);

    // 輸出日誌初始化信息
    log::debug!("Logger initialized with level: {}", log_level);
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
