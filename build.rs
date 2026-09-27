fn main() {
    for p in [
        "assets/windows.rc",
        "assets/usb-eye.ico",
        "assets/usb-glaz.manifest",
    ] {
        println!("cargo:rerun-if-changed={p}");
    }
    if std::env::var("TARGET").is_ok_and(|t| t == "x86_64-pc-windows-gnu") {
        let out =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("usb-glaz-res.o");
        let tool = std::env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into());
        let status = std::process::Command::new(tool)
            .args(["-I", ".", "-i", "assets/windows.rc", "-O", "coff", "-o"])
            .arg(&out)
            .status()
            .expect("Windows resource compiler");
        assert!(status.success(), "Windows resources failed");
        if std::env::var_os("CARGO_FEATURE_GUI").is_some() {
            println!("cargo:rustc-link-arg-bin=usb-doctor={}", out.display());
        }
    }
}
