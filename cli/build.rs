use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");
    if option_env!("TYPST_VERSION").is_some() {
        return;
    }

    let version = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout.get(..8)?.into()).ok())
        .unwrap_or_else(|| "(unknown version)".into());
    println!("cargo:rustc-env=TYPST_VERSION={version}");
}
