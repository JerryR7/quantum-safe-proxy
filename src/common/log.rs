//! Logging utility functions
//!
//! This module provides utility functions for the logging system.

/// Initialize the logging system
///
/// # Arguments
///
/// * `level` - Log level
pub fn init_logger(level: &str) {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", level);

    env_logger::init_from_env(env);
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
