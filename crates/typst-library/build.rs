use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8(output.stdout.strip_suffix(b"\n").unwrap().into()).ok()
        })
        .unwrap_or_else(|| "unknown hash".into());
    println!("cargo:rustc-env=TYPST_COMMIT={hash}");
}
