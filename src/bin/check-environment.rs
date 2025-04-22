//! Tool to check OpenSSL environment for post-quantum cryptography support

use std::process::exit;
use quantum_safe_proxy::crypto::{check_environment, diagnose_environment, IssueSeverity};

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    println!("=== Quantum Safe Proxy Environment Check ===\n");

    let env_info = check_environment();

    println!("OpenSSL version: {}", env_info.openssl_version);
    println!("Post-quantum cryptography available: {}", if env_info.pqc_available { "Yes" } else { "No" });

    if !env_info.supported_pq_algorithms.is_empty() {
        println!("\nSupported post-quantum algorithms:");
        for algo in &env_info.supported_pq_algorithms {
            println!("  - {}", algo);
        }
    }

    if !env_info.environment_variables.is_empty() {
        println!("\nEnvironment variables:");
        for (name, value) in &env_info.environment_variables {
            println!("  {}={}", name, value);
        }
    }

    let issues = diagnose_environment();

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
