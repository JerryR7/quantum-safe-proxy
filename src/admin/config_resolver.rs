//! Configuration Resolver Module
//!
//! This module provides functionality to resolve the effective configuration
//! from the ConfigManager and transform it into the ResolvedConfig format
//! used by the admin API.

use std::sync::Arc;
use chrono::Utc;
use serde_json::json;

use crate::config::types::ProxyConfig;
use crate::admin::types::{
    ResolvedConfig, ResolvedSetting, ConfigSource, SettingCategory,
    OperationalStatus
};
use crate::admin::error::AdminResult;

/// Resolve the current configuration into admin API format
pub fn resolve_config(config: Arc<ProxyConfig>) -> AdminResult<ResolvedConfig> {
    let mut settings = Vec::new();

    // Network settings
    settings.push(ResolvedSetting {
        name: "listen".to_string(),
        value: json!(config.listen().to_string()),
        source: map_value_source(config.source("listen")),
        hot_reloadable: false, // Requires restart (socket rebind)
        category: SettingCategory::Network,
        description: Some("Address and port to listen on for incoming connections".to_string()),
        security_affecting: false,
    });

    settings.push(ResolvedSetting {
        name: "target".to_string(),
        value: json!(config.target().to_string()),
        source: map_value_source(config.source("target")),
        hot_reloadable: false, // Requires restart (upstream address)
        category: SettingCategory::Network,
        description: Some("Target upstream server address and port".to_string()),
        security_affecting: false,
    });

    // Observability settings
    settings.push(ResolvedSetting {
        name: "log_level".to_string(),
        value: json!(config.log_level()),
        source: map_value_source(config.source("log_level")),
        hot_reloadable: true, // Can be changed at runtime
        category: SettingCategory::Observability,
        description: Some("Logging verbosity level (error, warn, info, debug, trace)".to_string()),
        security_affecting: false,
    });

    // Performance settings
    settings.push(ResolvedSetting {
        name: "buffer_size".to_string(),
        value: json!(config.buffer_size()),
        source: map_value_source(config.source("buffer_size")),
        hot_reloadable: true, // Can affect new connections
        category: SettingCategory::Performance,
        description: Some("Buffer size for data transfer in bytes".to_string()),
        security_affecting: false,
    });

    settings.push(ResolvedSetting {
        name: "connection_timeout".to_string(),
        value: json!(config.connection_timeout()),
        source: map_value_source(config.source("connection_timeout")),
        hot_reloadable: true, // Can affect new connections
        category: SettingCategory::Performance,
        description: Some("Connection timeout in seconds".to_string()),
        security_affecting: false,
    });

    // Authentication settings
    settings.push(ResolvedSetting {
        name: "client_cert_mode".to_string(),
        value: json!(config.client_cert_mode().to_string()),
        source: map_value_source(config.source("client_cert_mode")),
        hot_reloadable: false, // TLS acceptor created at startup, requires restart
        category: SettingCategory::Authentication,
        description: Some("Client certificate verification mode (required, optional, none)".to_string()),
        security_affecting: true, // Affects client authentication
    });

    // Security/TLS settings
    settings.push(ResolvedSetting {
        name: "cert".to_string(),
        value: json!(config.cert().display().to_string()),
        source: map_value_source(config.source("cert")),
        hot_reloadable: false, // TLS acceptor created at startup, requires restart
        category: SettingCategory::Security,
        description: Some("Path to primary (PQC/hybrid) TLS certificate".to_string()),
        security_affecting: true,
    });

    settings.push(ResolvedSetting {
        name: "key".to_string(),
        value: json!(config.key().display().to_string()),
        source: map_value_source(config.source("key")),
        hot_reloadable: false, // TLS acceptor created at startup, requires restart
        category: SettingCategory::Security,
        description: Some("Path to primary private key".to_string()),
        security_affecting: true,
    });

    if let Some(fallback_cert) = config.fallback_cert() {
        settings.push(ResolvedSetting {
            name: "fallback_cert".to_string(),
            value: json!(fallback_cert.display().to_string()),
            source: map_value_source(config.source("fallback_cert")),
            hot_reloadable: false, // TLS acceptor created at startup, requires restart
            category: SettingCategory::Security,
            description: Some("Path to fallback (classical) TLS certificate for non-PQC clients".to_string()),
            security_affecting: true,
        });
    }

    if let Some(fallback_key) = config.fallback_key() {
        settings.push(ResolvedSetting {
            name: "fallback_key".to_string(),
            value: json!(fallback_key.display().to_string()),
            source: map_value_source(config.source("fallback_key")),
            hot_reloadable: false, // TLS acceptor created at startup, requires restart
            category: SettingCategory::Security,
            description: Some("Path to fallback private key".to_string()),
            security_affecting: true,
        });
    }

    settings.push(ResolvedSetting {
        name: "client_ca_cert".to_string(),
        value: json!(config.client_ca_cert().display().to_string()),
        source: map_value_source(config.source("client_ca_cert")),
        hot_reloadable: false, // TLS acceptor created at startup, requires restart
        category: SettingCategory::Authentication,
        description: Some("Path to CA certificate for client certificate validation".to_string()),
        security_affecting: true,
    });

    // Dynamic certificate mode
    settings.push(ResolvedSetting {
        name: "dynamic_cert_enabled".to_string(),
        value: json!(config.has_fallback()),
        source: ConfigSource::Default, // Derived from fallback cert presence
        hot_reloadable: false,
        category: SettingCategory::Security,
        description: Some("Whether dynamic certificate selection is enabled (based on fallback cert configuration)".to_string()),
        security_affecting: false,
    });

    // OpenSSL directory (if configured)
    if let Some(openssl_dir) = config.openssl_dir() {
        settings.push(ResolvedSetting {
            name: "openssl_dir".to_string(),
            value: json!(openssl_dir.display().to_string()),
            source: map_value_source(config.source("openssl_dir")),
            hot_reloadable: false, // Requires restart
            category: SettingCategory::Security,
            description: Some("OpenSSL installation directory (advanced)".to_string()),
            security_affecting: false,
        });
    }

    // Get operational status
    let status = get_operational_status();

    Ok(ResolvedConfig {
        settings,
        status,
        resolved_at: Utc::now(),
        version: 1, // TODO: Track actual version from ConfigManager
    })
}

/// Map config value source to admin API source
fn map_value_source(source: &str) -> ConfigSource {
    match source {
        "command line" => ConfigSource::CommandLine,
        "environment" => ConfigSource::Environment,
        "file" => ConfigSource::File,
        "default" => ConfigSource::Default,
        _ => ConfigSource::Default,
    }
}

/// Get operational status of the proxy
///
/// TODO: This should integrate with actual metrics collection
fn get_operational_status() -> OperationalStatus {
    OperationalStatus::default()
}

/// Check if a setting is security-affecting
pub fn is_security_affecting(setting_name: &str) -> bool {
    matches!(
        setting_name,
        "client_cert_mode" | "cert" | "key" | "fallback_cert" | "fallback_key" | "client_ca_cert"
    )
}

/// Check if a setting can be hot-reloaded
pub fn is_hot_reloadable(setting_name: &str) -> bool {
    matches!(
        setting_name,
        "log_level" | "buffer_size" | "connection_timeout" | "client_cert_mode" |
        "cert" | "key" | "fallback_cert" | "fallback_key" | "client_ca_cert"
    )
}

/// Get setting category
pub fn get_setting_category(setting_name: &str) -> SettingCategory {
    match setting_name {
        "listen" | "target" => SettingCategory::Network,
        "cert" | "key" | "fallback_cert" | "fallback_key" | "openssl_dir" | "dynamic_cert_enabled" => {
            SettingCategory::Security
        }
        "buffer_size" | "connection_timeout" => SettingCategory::Performance,
        "log_level" => SettingCategory::Observability,
        "client_cert_mode" | "client_ca_cert" => SettingCategory::Authentication,
        _ => SettingCategory::Performance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_security_affecting() {
        assert!(is_security_affecting("client_cert_mode"));
        assert!(is_security_affecting("cert"));
        assert!(is_security_affecting("key"));
        assert!(!is_security_affecting("log_level"));
        assert!(!is_security_affecting("buffer_size"));
    }

    #[test]
    fn test_is_hot_reloadable() {
        // Performance settings - hot reloadable
        assert!(is_hot_reloadable("log_level"));
        assert!(is_hot_reloadable("buffer_size"));
        assert!(is_hot_reloadable("connection_timeout"));

        // Network settings - NOT hot reloadable (require restart)
        assert!(!is_hot_reloadable("listen"));
        assert!(!is_hot_reloadable("target"));

        // TLS/Auth settings - NOT hot reloadable (TLS acceptor created at startup)
        assert!(!is_hot_reloadable("client_cert_mode"));
        assert!(!is_hot_reloadable("cert"));
        assert!(!is_hot_reloadable("key"));
        assert!(!is_hot_reloadable("client_ca_cert"));
    }

    #[test]
    fn test_get_setting_category() {
        assert_eq!(get_setting_category("listen"), SettingCategory::Network);
        assert_eq!(get_setting_category("cert"), SettingCategory::Security);
        assert_eq!(get_setting_category("log_level"), SettingCategory::Observability);
        assert_eq!(get_setting_category("buffer_size"), SettingCategory::Performance);
        assert_eq!(get_setting_category("client_cert_mode"), SettingCategory::Authentication);
    }
}
