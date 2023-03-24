use std::process::Command;

fn get_version() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;

    String::from_utf8(output.stdout.get(..8)?.into()).ok()
}

fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");

    if std::env::var_os("TYPST_VERSION").is_some() {
        return;
    }

    let version = get_version().unwrap_or_else(|| "(unknown version)".into());

    println!("cargo:rustc-env=TYPST_VERSION={version}");
}
