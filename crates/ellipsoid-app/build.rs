//! Puts the application icon and version stamp into the Windows executable.
//!
//! Without this the binary carries no PE resources at all: Explorer, the
//! taskbar and Alt-Tab all show the generic executable icon, and the file's
//! Properties dialog is blank. The MSI's Start Menu shortcut has its own icon
//! and looks right either way, which is exactly why this is easy to miss —
//! everywhere *except* the Start Menu was unbranded.
//!
//! Windows targets only; everything else has nothing to do here.

fn main() {
    // The *target* OS, not the host: a Windows resource is meaningless in a
    // Linux or wasm binary, and `cfg!(windows)` in a build script describes the
    // machine doing the building.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Rerun only when the icon changes; the default is to rerun on any file
    // change in the crate, which would rebuild the app on every source edit.
    let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/icon.ico")
        .canonicalize()
        .expect("resources/icon.ico is part of the repository");
    println!("cargo::rerun-if-changed={}", icon.display());
    println!("cargo::rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(icon.to_str().expect("repository path is UTF-8"));
        // FileVersion and ProductVersion come from CARGO_PKG_VERSION already.
        resource.set("ProductName", "Ellipsoid Pattern Generator");
        resource.set(
            "FileDescription",
            "Generate cut patterns for ellipsoid shapes",
        );
        // Matches `Manufacturer` in wix/main.wxs, so the Properties dialog and
        // the installer's Programs-and-Features entry agree.
        resource.set("CompanyName", "P. Spindler");
        // Deliberately loud: a release that silently ships an unbranded binary
        // is the failure this file exists to prevent. `rc.exe` ships with the
        // Windows SDK, which an MSVC toolchain already needs for `link.exe`.
        resource
            .compile()
            .expect("embedding the Windows icon and version resource");
    }
}
