// build.rs
fn main() {
    // Allow users to specify OpenSSL path with OPENSSL_DIR environment variable
    // If not set, fallback to /opt/openssl35
    let openssl_dir = std::env::var("OPENSSL_DIR")
        .unwrap_or_else(|_| "/opt/openssl35".into());
    let libdir = format!("{}/lib", &openssl_dir);

    // Tell rustc where to find .so files
    println!("cargo:rustc-link-search=native={}", libdir);
    // If your .so files are in lib64 instead of lib, add this line:
    println!("cargo:rustc-link-search=native={}/lib64", openssl_dir);

    // Specify libraries to link
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");

    // Critical: Write RPATH into the ELF header
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir);
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib64", openssl_dir);

    // Recompilation conditions
    println!("cargo:rerun-if-env-changed=OPENSSL_DIR");
}
