use std::process::Command;

fn main() {
    let version = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout[..8].into()).ok())
        .unwrap_or_else(|| "(unknown version)".into());
    println!("cargo:rustc-env=TYPST_VERSION={version}");
}
