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

        Self {
            // Network settings - always override
            listen: other.listen,
            target: other.target,

            // General settings
            log_level: merge_field(&self.log_level, &other.log_level, &default.log_level, false),
            client_cert_mode: other.client_cert_mode.clone(), // Enum - directly override
            buffer_size: other.buffer_size,           // Numeric - directly override
            connection_timeout: other.connection_timeout, // Numeric - directly override
            openssl_dir: merge_option(&self.openssl_dir, &other.openssl_dir),

            // Certificate strategy settings
            strategy: other.strategy.clone(),  // Enum - directly override
            traditional_cert: merge_field(
                &self.traditional_cert,
                &other.traditional_cert,
                &default.traditional_cert,
                false,
            ),
            traditional_key: merge_field(
                &self.traditional_key,
                &other.traditional_key,
                &default.traditional_key,
                false,
            ),
            hybrid_cert: merge_field(
                &self.hybrid_cert,
                &other.hybrid_cert,
                &default.hybrid_cert,
                false,
            ),
            hybrid_key: merge_field(
                &self.hybrid_key,
                &other.hybrid_key,
                &default.hybrid_key,
                false,
            ),
            pqc_only_cert: merge_option(&self.pqc_only_cert, &other.pqc_only_cert),
            pqc_only_key: merge_option(&self.pqc_only_key, &other.pqc_only_key),
            client_ca_cert_path: merge_field(
                &self.client_ca_cert_path,
                &other.client_ca_cert_path,
                &default.client_ca_cert_path,
                false,
            ),
        }
    }
}
