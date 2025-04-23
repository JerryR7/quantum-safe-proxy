//! Common module
//!
//! This module contains shared types, errors, and utility functions used throughout the application.

pub mod error;
pub mod types;
pub mod fs;
pub mod log;
pub mod net;

// Re-export commonly used types and functions
pub use error::{ProxyError, Result};
pub use fs::{check_file_exists, read_file};
pub use log::init_logger;
pub use net::parse_socket_addr;
