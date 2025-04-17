//! Environment check tool
//!
//! This tool checks the environment for OpenSSL with post-quantum cryptography support.
//! It supports both OpenSSL 3.5+ with built-in PQC and older versions with OQS Provider.

use std::process::exit;
use quantum_safe_proxy::crypto::provider::{check_environment, diagnose_environment, IssueSeverity};

fn main() {
    // Initialize logger
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    println!("=== Quantum Safe Proxy Environment Check ===\n");

    // Check environment
    let env_info = check_environment();

    // Print environment information
    println!("OpenSSL version: {}", env_info.openssl_version);
    println!("Post-quantum cryptography available: {}", if env_info.pqc_available { "Yes" } else { "No" });

    // Print key exchange algorithms
    if !env_info.key_exchange_algorithms.is_empty() {
        println!("\nSupported post-quantum key exchange algorithms:");
        for algo in &env_info.key_exchange_algorithms {
            println!("  - {}", algo);
        }
    }

    // Print signature algorithms
    if !env_info.signature_algorithms.is_empty() {
        println!("\nSupported post-quantum signature algorithms:");
        for algo in &env_info.signature_algorithms {
            println!("  - {}", algo);
        }
    }

    // Print environment variables
    if !env_info.env_vars.is_empty() {
        println!("\nEnvironment variables:");
        for (name, value) in &env_info.env_vars {
            println!("  {}={}", name, value);
        }
    }

    // Diagnose environment issues
    let issues = diagnose_environment(&env_info);

    if !issues.is_empty() {
        println!("\nEnvironment issues:");

        let mut has_errors = false;

        for issue in &issues {
            let prefix = match issue.severity {
                IssueSeverity::Info => "INFO",
                IssueSeverity::Warning => "WARNING",
                IssueSeverity::Error => {
                    has_errors = true;
                    "ERROR"
                },
            };

            println!("  [{:7}] {}", prefix, issue.message);
        }

        if has_errors {
            println!("\nCritical issues were found. Please resolve them before using the proxy.");
            exit(1);
        }
    } else {
        println!("\nNo issues found. Environment is ready for quantum-safe proxy.");
    }

    // Print summary
    println!("\n=== Summary ===");
    if env_info.pqc_available {
        println!("✅ Post-quantum cryptography support is available and properly configured.");
        println!("✅ Quantum-safe TLS connections are enabled.");
    } else {
        println!("⚠️  Post-quantum cryptography support is NOT available.");
        println!("⚠️  Quantum-safe TLS connections are NOT enabled.");
        println!("\nTo enable post-quantum support, install OpenSSL 3.5+ with built-in post-quantum capabilities:");
        println!("  1. Use the provided Docker image: docker/Dockerfile.openssl35");
        println!("  2. Or install OpenSSL 3.5+ manually and set the environment variables");
    }

    println!("\nFor more information, visit: https://github.com/openssl/openssl");
}
