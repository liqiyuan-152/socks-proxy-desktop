fn main() {
    #[cfg(windows)]
    {
        // Rust test executables do not inherit Tauri's application resource.
        // WebView/dialog dependencies import TaskDialogIndirect from the v6
        // common controls assembly, which must be declared at load time.
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-test.manifest");
        println!("cargo:rerun-if-changed=windows-test.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    #[cfg(windows)]
    {
        // The linker embeds the same Common Controls v6 manifest in all
        // executables, including unit tests. Do not also put a manifest in
        // Tauri's resource.lib (which would produce duplicate resource #1).
        let attributes = tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        tauri_build::try_build(attributes).expect("failed to build Tauri resources");
    }
    #[cfg(not(windows))]
    tauri_build::build()
}
