//! Environment check tool
//!
//! This tool checks the environment for OQS-OpenSSL and other dependencies.

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
    println!("OQS-OpenSSL available: {}", if env_info.oqs_available { "Yes" } else { "No" });
    
    if let Some(path) = &env_info.oqs_path {
        println!("OQS-OpenSSL path: {}", path.display());
    }
    
    println!("\nSupported providers:");
    for provider in &env_info.supported_providers {
        println!("  - {:?}", provider);
    }
    
    if !env_info.available_pqc_algorithms.is_empty() {
        println!("\nAvailable post-quantum algorithms:");
        for algo in &env_info.available_pqc_algorithms {
            println!("  - {}", algo);
        }
    } else if env_info.oqs_available {
        println!("\nNo post-quantum algorithms detected, but OQS-OpenSSL is available.");
    }
    
    // Diagnose environment issues
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
            
            if let Some(resolution) = &issue.resolution {
                println!("             Resolution: {}", resolution);
            }
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
    if env_info.oqs_available {
        println!("✅ OQS-OpenSSL is available and properly configured.");
        println!("✅ Post-quantum cryptography support is enabled.");
    } else {
        println!("⚠️  OQS-OpenSSL is NOT available.");
        println!("⚠️  Post-quantum cryptography support is NOT enabled.");
        println!("\nTo enable post-quantum support, install OQS-OpenSSL:");
        println!("  1. Run the installation script: ./scripts/install-oqs.sh");
        println!("  2. Set the environment variables as instructed");
    }
    
    println!("\nFor more information, visit: https://github.com/open-quantum-safe/openssl");
}
