//! Configuration traits
//!
//! This module defines traits for configuration operations.

use std::path::Path;
use crate::common::Result;
use crate::config::types::ProxyConfig;

/// Trait for loading configuration
pub trait ConfigLoader {
    /// Load configuration from a file
    ///
    /// This method loads configuration from a file, environment variables, and defaults,
    /// then validates the configuration before returning it.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> where Self: Sized;

    /// Auto-detect and load configuration from the best available source
    ///
    /// This method loads configuration with proper priority:
    /// 1. Default values (lowest priority)
    /// 2. Configuration file (config.json by default)
    /// 3. Environment variables
    /// 4. Command line arguments (highest priority)
    fn auto_load() -> Result<Self> where Self: Sized;
}

/// Trait for validating configuration
pub trait ConfigValidator {
    /// Validate configuration
    ///
    /// Checks if certificate files exist and other configuration is valid.
    /// Returns an error if the configuration is invalid.
    fn validate(&self) -> Result<()>;

    /// Check configuration for potential issues
    ///
    /// This method checks the configuration for potential issues and returns
    /// a list of warnings. Unlike `validate()`, this method does not return
    /// an error if issues are found.
    fn check(&self) -> Vec<String>;
}

/// Trait for merging configuration
pub trait ConfigMerger {
    /// Merge another configuration into this one
    ///
    /// Values from `other` will override values in `self` if they are not the default values.
    /// This is used to implement the configuration priority system.
    fn merge(&self, other: impl AsRef<ProxyConfig>) -> Self where Self: Sized;
}

/// Trait for logging configuration
pub trait ConfigLogger {
    /// Log the configuration with source information
    fn log(&self);
}
