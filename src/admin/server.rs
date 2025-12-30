//! Admin HTTP Server Module
//!
//! This module sets up the HTTP server for the admin API using axum.

use std::net::SocketAddr;
use axum::{
    Router,
    routing::{get, post, patch},
    middleware,
};
use tower_http::trace::TraceLayer;

use crate::admin::auth::{AuthState, auth_middleware};
use crate::admin::handlers;
use crate::admin::error::AdminResult;
use crate::admin::types::ApiKey;

/// Admin server configuration
#[derive(Debug, Clone)]
pub struct AdminServerConfig {
    /// Listen address for admin API
    pub listen_addr: SocketAddr,

    /// API keys for authentication
    pub api_keys: Vec<ApiKey>,

    /// Audit log file path
    pub audit_log_path: String,
}

impl Default for AdminServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8443".parse().unwrap(),
            api_keys: Vec::new(),
            audit_log_path: "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string(),
        }
    }
}

/// Start the admin HTTP server
pub async fn start_admin_server(config: AdminServerConfig) -> AdminResult<()> {
    // Create authentication state
    let auth_state = AuthState::new(config.api_keys);

    // Build application router
    let app = build_router(auth_state);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    log::info!("Admin API server listening on {}", config.listen_addr);

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::admin::error::AdminError::Internal(e.to_string()))?;

    Ok(())
}

/// Build the application router with all routes
fn build_router(auth_state: AuthState) -> Router {
    Router::new()
        // Public health check (no auth required)
        .route("/health", get(handlers::health_check))

        // Configuration endpoints
        .route("/api/config", get(handlers::get_config))
        .route("/api/config", patch(handlers::patch_config))
        .route("/api/config/rollback", post(handlers::rollback_config))
        .route("/api/config/export", post(handlers::export_config))
        .route("/api/config/import", post(handlers::import_config))

        // Status endpoint
        .route("/api/status", get(handlers::get_status))

        // Audit endpoints
        .route("/api/audit", get(handlers::get_audit_log))
        .route("/api/audit/:id", get(handlers::get_audit_entry))
        .route("/api/audit/export", post(handlers::export_audit_log))

        // UI endpoint
        .route("/", get(handlers::serve_ui))

        // Add authentication middleware to all /api routes
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ))

        // Add tracing
        .layer(TraceLayer::new_for_http())

        // Add state
        .with_state(auth_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AdminServerConfig::default();
        assert_eq!(config.listen_addr.port(), 8443);
        assert!(config.api_keys.is_empty());
    }
}
