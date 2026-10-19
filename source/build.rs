use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=resources/clock.rc");
    println!("cargo:rerun-if-changed=resources/clock.ico");
    println!("cargo:rerun-if-changed=resources/clock.manifest");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("clock.res");
    let sdk = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let rc = env::var_os("RC")
        .map(PathBuf::from)
        .or_else(|| {
            let mut versions: Vec<_> = std::fs::read_dir(&sdk)
                .ok()?
                .filter_map(Result::ok)
                .map(|v| v.path())
                .filter(|p| p.join("x64/rc.exe").exists())
                .collect();
            versions.sort();
            versions.pop().map(|p| p.join("x64/rc.exe"))
        })
        .unwrap_or_else(|| "rc.exe".into());
    let status = Command::new(rc)
        .args(["/nologo", "/c65001", "/fo"])
        .arg(&out)
        .arg("resources/clock.rc")
        .status()
        .expect("Windows SDK resource compiler is required");
    assert!(status.success(), "Resource compilation failed");
    println!("cargo:rustc-link-arg={}", out.display());
}
