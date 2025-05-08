//! Configuration loader implementation
//!
//! This module provides functionality for loading configuration from different sources.

use std::path::Path;
use log::{debug, warn};

use crate::common::Result;
use crate::config::{ENV_PREFIX, ProxyConfig};
use crate::config::builder::ConfigBuilder;
use crate::config::traits::ConfigLoader;

impl ConfigLoader for ProxyConfig {
    /// Auto-detect and load configuration from the best available source
    ///
    /// This method loads configuration with proper priority:
    /// 1. Default values (lowest priority)
    /// 2. Configuration file (config.json by default)
    /// 3. Environment variables
    /// 4. Command line arguments (highest priority)
    fn auto_load() -> Result<Self> {
        // Get command line arguments
        let args: Vec<String> = std::env::args().collect();

        // Use the builder's auto_load function
        match crate::config::builder::auto_load(args) {
            Ok(config) => Ok(config),
            Err(e) => Err(e.into())
        }
    }

    /// Load configuration from a specific file
    ///
    /// This method loads configuration from a file, environment variables, and defaults,
    /// then validates the configuration before returning it.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            warn!("Configuration file not found: {}", path.display());
            warn!("Will use default values unless overridden by environment variables");
        } else {
            debug!("Using configuration file: {}", path.display());
        }

        // Use the builder to load configuration
        let config = ConfigBuilder::new()
            .with_defaults()
            .with_file(path)
            .with_env(ENV_PREFIX)
            .build()?;

        debug!("Configuration validated successfully");
        Ok(config)
    }
}




