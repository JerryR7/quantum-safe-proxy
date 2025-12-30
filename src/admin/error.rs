//! Admin API Error Types
//!
//! This module defines error types specific to the admin API,
//! including validation errors, authentication failures, and persistence errors.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response, Json};
use serde::{Serialize, Deserialize};

/// Result type for admin API operations
pub type AdminResult<T> = Result<T, AdminError>;

/// Admin API error types
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    /// Configuration validation failed
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization failed (insufficient permissions)
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Configuration persistence error
    #[error("Persistence error: {0}")]
    Persistence(String),

    /// Audit log error
    #[error("Audit log error: {0}")]
    AuditLog(String),

    /// Configuration manager error
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Bad request
    #[error("Bad request: {0}")]
    BadRequest(String),
}

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub message: String,

    /// Optional detailed error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let (status, message, details) = match &self {
            AdminError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                "Validation failed".to_string(),
                Some(msg.clone()),
            ),
            AdminError::Authentication(msg) => (
                StatusCode::UNAUTHORIZED,
                "Authentication failed".to_string(),
                Some(msg.clone()),
            ),
            AdminError::Authorization(msg) => (
                StatusCode::FORBIDDEN,
                "Insufficient permissions".to_string(),
                Some(msg.clone()),
            ),
            AdminError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "Resource not found".to_string(),
                Some(msg.clone()),
            ),
            AdminError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "Bad request".to_string(),
                Some(msg.clone()),
            ),
            AdminError::Persistence(msg) |
            AdminError::AuditLog(msg) |
            AdminError::Config(msg) |
            AdminError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
                Some(msg.clone()),
            ),
            AdminError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "IO error".to_string(),
                Some(e.to_string()),
            ),
            AdminError::Serialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Serialization error".to_string(),
                Some(e.to_string()),
            ),
        };

        let error_response = ErrorResponse { message, details };
        (status, Json(error_response)).into_response()
    }
}

impl From<crate::common::ProxyError> for AdminError {
    fn from(err: crate::common::ProxyError) -> Self {
        AdminError::Config(err.to_string())
    }
}
