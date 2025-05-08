//! Configuration traits
//!
//! This module defines traits for configuration loading and validation.

use std::path::Path;
use crate::common::Result;

/// Configuration loader trait
///
/// This trait defines methods for loading configuration from different sources.
pub trait ConfigLoader: Sized {
    /// Auto-detect and load configuration from the best available source
    ///
    /// This method loads configuration with proper priority:
    /// 1. Default values (lowest priority)
    /// 2. Configuration file (config.json by default)
    /// 3. Environment variables
    /// 4. Command line arguments (highest priority)
    fn auto_load() -> Result<Self>;
    
    /// Load configuration from a specific file
    ///
    /// This method loads configuration from a file, environment variables, and defaults,
    /// then validates the configuration before returning it.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self>;
}

/// Configuration validator trait
///
/// This trait defines methods for validating configuration.
pub trait ConfigValidator {
    /// Validate the configuration
    ///
    /// This method checks if the configuration is valid and returns an error if not.
    fn validate(&self) -> Result<()>;
}
