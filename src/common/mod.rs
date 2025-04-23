//! Common module
//!
//! This module contains shared types, errors, and utility functions used throughout the application.

pub mod error;
pub mod log;
pub mod buffer_pool;

// Re-export commonly used types and functions
pub use error::{ProxyError, Result};
pub use log::init_logger;
pub use buffer_pool::{BufferPool, PooledBuffer};
