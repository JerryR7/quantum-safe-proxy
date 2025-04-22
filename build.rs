// build.rs
fn main() {
    // 允許使用者用環境變數 OPENSSL_DIR 指定路徑，
    // 如果沒設定就 fallback 到 /opt/openssl-3.5.0
    let openssl_dir = std::env::var("OPENSSL_DIR")
        .unwrap_or_else(|_| "/opt/openssl-3.5.0".into());
    let libdir = format!("{}/lib", &openssl_dir);

    // 告訴 rustc 連哪裡去找 .so
    println!("cargo:rustc-link-search=native={}", libdir);
    // 如果你的 so 在 lib64 而不是 lib，也可以多加一行：
    println!("cargo:rustc-link-search=native={}/lib64", openssl_dir);

    // 指定要 link 的 library
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");

    // 最關鍵：把 RPATH 寫進 ELF header
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir);
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib64", openssl_dir);
    
    // 重新編譯條件
    println!("cargo:rerun-if-env-changed=OPENSSL_DIR");
}
