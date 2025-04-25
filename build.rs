// build.rs - OpenSSL 3.5+ detection and configuration
use std::path::Path;
use std::process::Command;
use std::env;

/// Check if OpenSSL version is 3.5 or higher
fn is_openssl_35_or_higher(openssl_bin: &Path) -> bool {
    Command::new(openssl_bin)
        .arg("version")
        .output()
        .ok()
        .and_then(|output| {
            let version = String::from_utf8_lossy(&output.stdout);
            // Parse version string like "OpenSSL 3.5.0 ..."
            version.split_whitespace().nth(1).and_then(|ver_str| {
                let parts: Vec<&str> = ver_str.split('.').collect();
                if parts.len() >= 2 {
                    parts[0].parse::<u32>().ok().and_then(|major| {
                        parts[1].parse::<u32>().ok().map(|minor| {
                            // Check if version is 3.5 or higher
                            (major == 3 && minor >= 5) || major > 3
                        })
                    })
                } else {
                    None
                }
            })
        })
        .unwrap_or(false)
}

fn main() {
    // Common OpenSSL 3.5+ installation locations
    let openssl35_locations = [
        "/opt/openssl-3.5.0", "/opt/openssl35",
        "/usr/local/openssl-3.5.0", "/usr/local/openssl35",
        "/usr/local/opt/openssl-3.5.0", "/usr/local/opt/openssl35",
    ];

    // Get OpenSSL directory from environment or auto-detect
    let openssl_dir = env::var("OPENSSL_DIR").unwrap_or_else(|_| {
        // Try to auto-detect OpenSSL 3.5+
        openssl35_locations.iter()
            .find_map(|&location| {
                let path = Path::new(location);
                let openssl_bin = path.join("bin").join("openssl");

                if path.exists() && openssl_bin.exists() && is_openssl_35_or_higher(&openssl_bin) {
                    println!("cargo:warning=Detected OpenSSL 3.5+ at: {}", location);
                    Some(location.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                println!("cargo:warning=OpenSSL 3.5+ not found, using default OpenSSL");
                "/opt/openssl".to_string()
            })
    });

    println!("cargo:warning=Using OpenSSL directory: {}", openssl_dir);

    // Configure library paths
    let lib_dir = Path::new(&openssl_dir).join("lib");
    let lib64_dir = Path::new(&openssl_dir).join("lib64");
    let include_dir = Path::new(&openssl_dir).join("include");

    // Set library search paths
    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    if lib64_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib64_dir.display());
    }

    // Set environment variables
    println!("cargo:rustc-env=OPENSSL_DIR={}", openssl_dir);

    if lib64_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_LIB_DIR={}", lib64_dir.display());
    } else if lib_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_LIB_DIR={}", lib_dir.display());
    }

    if include_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_INCLUDE_DIR={}", include_dir.display());
    }

    // Specify libraries to link
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");

    // Set feature flags if OpenSSL 3.5+ is detected
    let openssl_bin = Path::new(&openssl_dir).join("bin").join("openssl");
    if openssl_bin.exists() && is_openssl_35_or_higher(&openssl_bin) {
        println!("cargo:rustc-cfg=feature=\"openssl35\"");
        println!("cargo:warning=Enabled openssl35 feature flag");
        println!("cargo:rustc-env=USE_HYBRID_CERT_DEFAULT=true");
    }

    // Tell Cargo to rerun if OpenSSL files or environment variables change
    println!("cargo:rerun-if-changed={}", openssl_dir);
    println!("cargo:rerun-if-env-changed=OPENSSL_DIR");
}
