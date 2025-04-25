//! Proxy service module
//!
//! This module implements the core functionality of the proxy service,
//! including TLS connection handling and data forwarding.
//!
//! The proxy service uses a message-driven architecture to avoid deadlocks
//! and provide better separation of concerns. It leverages Rust's trait system
//! and ownership model to provide a clean, lock-free implementation.

pub mod server;
mod handler;
mod forwarder;
mod message;
mod service;

// Legacy export for backward compatibility
pub use server::Proxy;

// New message-driven architecture exports
pub use message::{ProxyMessage, ProxyHandle};
pub use service::{ProxyService, StandardProxyService, ConnectionInfo};
