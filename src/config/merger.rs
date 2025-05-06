//! Configuration merging functionality
//!
//! This module provides functionality for merging configurations from different sources.

use crate::config::ProxyConfig;

/// Trait for merging configurations
pub trait ConfigMerger {
    /// Merge another configuration into this one
    ///
    /// Values from `other` will override values in `self` if they are not the default values.
    /// This is used to implement the configuration priority system.
    fn merge(&self, other: impl AsRef<Self>) -> Self where Self: Sized;
}

impl ConfigMerger for ProxyConfig {
    fn merge(&self, other: impl AsRef<Self>) -> Self {
        let other = other.as_ref();
        let default = Self::default();

        // Helper function to merge a field based on a condition
        fn merge_field<T: Clone + PartialEq>(
            self_val: &T,
            other_val: &T,
            default_val: &T,
            override_default: bool,
        ) -> T {
            if override_default || other_val != default_val {
                other_val.clone()
            } else {
                self_val.clone()
            }
        }

        // Helper function to merge an Option<T> field
        fn merge_option<T: Clone>(self_val: &Option<T>, other_val: &Option<T>) -> Option<T> {
            if other_val.is_some() {
                other_val.clone()
            } else {
                self_val.clone()
            }
        }

        // Create a new configuration with merged values
        // The merge logic is:
        // 1. If other value is not default, use it (higher priority source)
        // 2. Otherwise, keep the current value (lower priority source)

        // For network settings, we always use the other value if it's different from default
        let listen = if other.listen != default.listen {
            other.listen
        } else {
            self.listen
        };

        let target = if other.target != default.target {
            other.target.clone()
        } else {
            self.target.clone()
        };

        // For general settings, use merge_field helper
        let log_level = merge_field(&self.log_level, &other.log_level, &default.log_level, false);

        // For enums, check if other value is different from default
        let client_cert_mode = if other.client_cert_mode != default.client_cert_mode {
            other.client_cert_mode.clone()
        } else {
            self.client_cert_mode.clone()
        };

        // For numeric values, check if other value is different from default
        let buffer_size = if other.buffer_size != default.buffer_size {
            other.buffer_size
        } else {
            self.buffer_size
        };

        let connection_timeout = if other.connection_timeout != default.connection_timeout {
            other.connection_timeout
        } else {
            self.connection_timeout
        };

        // For options, use merge_option helper
        let openssl_dir = merge_option(&self.openssl_dir, &other.openssl_dir);

        // For certificate strategy settings
        let strategy = if other.strategy != default.strategy {
            other.strategy.clone()
        } else {
            self.strategy.clone()
        };

        // For certificate paths, use merge_field helper
        let traditional_cert = merge_field(
            &self.traditional_cert,
            &other.traditional_cert,
            &default.traditional_cert,
            false,
        );

        let traditional_key = merge_field(
            &self.traditional_key,
            &other.traditional_key,
            &default.traditional_key,
            false,
        );

        let hybrid_cert = merge_field(
            &self.hybrid_cert,
            &other.hybrid_cert,
            &default.hybrid_cert,
            false,
        );

        let hybrid_key = merge_field(
            &self.hybrid_key,
            &other.hybrid_key,
            &default.hybrid_key,
            false,
        );

        let pqc_only_cert = merge_option(&self.pqc_only_cert, &other.pqc_only_cert);
        let pqc_only_key = merge_option(&self.pqc_only_key, &other.pqc_only_key);

        let client_ca_cert_path = merge_field(
            &self.client_ca_cert_path,
            &other.client_ca_cert_path,
            &default.client_ca_cert_path,
            false,
        );

        Self {
            // Network settings
            listen,
            target,

            // General settings
            log_level,
            client_cert_mode,
            buffer_size,
            connection_timeout,
            openssl_dir,

            // Certificate strategy settings
            strategy,
            traditional_cert,
            traditional_key,
            hybrid_cert,
            hybrid_key,
            pqc_only_cert,
            pqc_only_key,
            client_ca_cert_path,
        }
    }
}
