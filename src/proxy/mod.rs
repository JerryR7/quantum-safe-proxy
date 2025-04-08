//! Proxy service module
//!
//! This module implements the core functionality of the proxy service,
//! including TLS connection handling and data forwarding.

mod server;
mod handler;
mod forwarder;

pub use server::Proxy;
