//! Authentication and Authorization Module
//!
//! This module provides API key authentication and RBAC (Role-Based Access Control)
//! for the admin API.

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::Utc;

use crate::admin::types::{ApiKey, Role, AuditAction};
use crate::admin::error::{AdminError, AdminResult};

/// Authentication state shared across handlers
#[derive(Debug, Clone)]
pub struct AuthState {
    /// API keys for authentication
    pub api_keys: Arc<Vec<ApiKey>>,
}

impl AuthState {
    /// Create a new authentication state
    pub fn new(api_keys: Vec<ApiKey>) -> Self {
        Self {
            api_keys: Arc::new(api_keys),
        }
    }

    /// Validate an API key and return the associated role
    pub fn validate_api_key(&self, key: &str) -> Option<(String, Role)> {
        for api_key in self.api_keys.iter() {
            // Constant-time comparison to prevent timing attacks
            if constant_time_compare(&api_key.key, key) {
                // Check expiration
                if let Some(expires_at) = api_key.expires_at {
                    if Utc::now() > expires_at {
                        log::warn!(
                            "Authentication attempt with expired API key for user: {}",
                            api_key.name
                        );
                        log_auth_event(AuditAction::AuthFailure, &api_key.name, "Expired API key");
                        return None; // Expired key
                    }
                }

                log::info!(
                    "Successful authentication for user: {} with role: {:?}",
                    api_key.name,
                    api_key.role
                );
                return Some((api_key.name.clone(), api_key.role));
            }
        }

        None
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }

    result == 0
}

/// Authenticated user information
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// Username/operator name
    pub name: String,

    /// User's role
    pub role: Role,
}

/// Authentication middleware
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AdminError> {
    // Extract Authorization header
    let headers = req.headers();
    let auth_header = headers
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if auth_header.is_none() {
        log::warn!("Authentication failure: Missing Authorization header from {}",
            req.uri().path()
        );
        log_auth_event(AuditAction::AuthFailure, "unknown", "Missing Authorization header");
        return Err(AdminError::Authentication("Missing Authorization header".to_string()));
    }

    let auth_header = auth_header.unwrap();

    // Parse Bearer token
    let token = auth_header.strip_prefix("Bearer ");

    if token.is_none() {
        log::warn!("Authentication failure: Invalid Authorization header format from {}",
            req.uri().path()
        );
        log_auth_event(AuditAction::AuthFailure, "unknown", "Invalid Authorization header format");
        return Err(AdminError::Authentication("Invalid Authorization header format".to_string()));
    }

    let token = token.unwrap();

    // Validate API key
    let validation_result = auth_state.validate_api_key(token);

    if validation_result.is_none() {
        log::warn!("Authentication failure: Invalid API key attempt from {}",
            req.uri().path()
        );
        log_auth_event(AuditAction::AuthFailure, "unknown", "Invalid API key");
        return Err(AdminError::Authentication("Invalid API key".to_string()));
    }

    let (name, role) = validation_result.unwrap();

    // Insert authenticated user into request extensions
    let auth_user = AuthUser { name, role };
    req.extensions_mut().insert(auth_user);

    // Continue to next handler
    Ok(next.run(req).await)
}

/// Require specific role for endpoint access
pub fn require_role(user: &AuthUser, required_role: Role) -> AdminResult<()> {
    if user.role >= required_role {
        Ok(())
    } else {
        log::warn!(
            "Authorization failure: User {} (role: {:?}) attempted to access endpoint requiring {:?}",
            user.name,
            user.role,
            required_role
        );
        log_auth_event(
            AuditAction::AuthzFailure,
            &user.name,
            &format!("Insufficient permissions: {:?} required, but user has {:?}", required_role, user.role)
        );
        Err(AdminError::Authorization(format!(
            "Insufficient permissions: {:?} required, but user has {:?}",
            required_role, user.role
        )))
    }
}

/// Check if user can modify security settings
pub fn can_modify_security_settings(user: &AuthUser) -> bool {
    let can_modify = user.role >= Role::Admin;

    if !can_modify {
        log::warn!(
            "Authorization check: User {} (role: {:?}) cannot modify security settings (Admin role required)",
            user.name,
            user.role
        );
    }

    can_modify
}

/// Log authentication/authorization event
///
/// This logs auth events to the application log. The audit log module
/// (src/admin/audit.rs) handles persisting these to the audit trail.
fn log_auth_event(action: AuditAction, operator: &str, message: &str) {
    match action {
        AuditAction::AuthFailure => {
            log::warn!(
                "AUTH_EVENT: Authentication failure - operator: {}, reason: {}",
                operator,
                message
            );
        }
        AuditAction::AuthzFailure => {
            log::warn!(
                "AUTH_EVENT: Authorization failure - operator: {}, reason: {}",
                operator,
                message
            );
        }
        _ => {
            log::info!(
                "AUTH_EVENT: {} - operator: {}, details: {}",
                match action {
                    AuditAction::ConfigChange => "Config change",
                    AuditAction::ConfigExport => "Config export",
                    AuditAction::ConfigImportPreview => "Config import preview",
                    AuditAction::ConfigImportApply => "Config import apply",
                    AuditAction::ConfigRollback => "Config rollback",
                    _ => "Unknown action",
                },
                operator,
                message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("secret", "secret"));
        assert!(!constant_time_compare("secret", "public"));
        assert!(!constant_time_compare("short", "longer"));
    }

    #[test]
    fn test_api_key_validation() {
        let api_keys = vec![
            ApiKey {
                key: "valid-key-123".to_string(),
                role: Role::Admin,
                name: "admin-user".to_string(),
                expires_at: None,
            },
        ];

        let auth_state = AuthState::new(api_keys);

        // Valid key
        let result = auth_state.validate_api_key("valid-key-123");
        assert!(result.is_some());
        let (name, role) = result.unwrap();
        assert_eq!(name, "admin-user");
        assert_eq!(role, Role::Admin);

        // Invalid key
        let result = auth_state.validate_api_key("invalid-key");
        assert!(result.is_none());
    }

    #[test]
    fn test_require_role() {
        let admin_user = AuthUser {
            name: "admin".to_string(),
            role: Role::Admin,
        };

        let operator_user = AuthUser {
            name: "operator".to_string(),
            role: Role::Operator,
        };

        // Admin can access admin endpoints
        assert!(require_role(&admin_user, Role::Admin).is_ok());

        // Operator cannot access admin endpoints
        assert!(require_role(&operator_user, Role::Admin).is_err());

        // Operator can access operator endpoints
        assert!(require_role(&operator_user, Role::Operator).is_ok());
    }
}
