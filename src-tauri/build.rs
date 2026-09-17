fn main() {
    // Tauri embeds a Windows app manifest (it needs Common Controls v6) — but only into the app
    // binary. Test executables link the same code without it and die at startup with
    // STATUS_ENTRYPOINT_NOT_FOUND. So we opt out of Tauri's manifest and link our own copy into
    // every target, tests included.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run tauri-build");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        // Turn linker warnings about the manifest into errors rather than a broken binary.
        println!("cargo:rustc-link-arg=/WX");
    }
}
