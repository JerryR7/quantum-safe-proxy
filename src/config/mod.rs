//! 配置處理模組
//!
//! 這個模組處理應用程序的配置，包括命令行參數和配置文件。

mod config;

pub use config::ProxyConfig;
// 注意：現在從 common::net 模組導出 parse_socket_addr
