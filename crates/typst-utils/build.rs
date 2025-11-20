use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");
    println!("cargo:rerun-if-env-changed=TYPST_COMMIT_SHA");

    if option_env!("TYPST_VERSION").is_none() {
        println!("cargo:rustc-env=TYPST_VERSION={}", env!("CARGO_PKG_VERSION"));
    }

    if option_env!("TYPST_COMMIT_SHA").is_none()
        && let Some(sha) = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout.get(..8)?.into()).ok())
    {
        println!("cargo:rustc-env=TYPST_COMMIT_SHA={sha}");
    }
}
