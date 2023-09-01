use std::borrow::Cow;
use std::process::Command;

fn main() {
    // only set version if not overridden by env variable
    println!("cargo:rerun-if-env-changed=TYPST_COMMIT");
    let pkg = env!("CARGO_PKG_VERSION");
    let hash = option_env!("TYPST_COMMIT").map(Cow::Borrowed).unwrap_or_else(|| {
        let hash = typst_commit();
        println!("cargo:rustc-env=TYPST_COMMIT={hash}");
        Cow::Owned(hash)
    });
    if hash.is_empty() {
        println!("cargo:rustc-env=TYPST_VERSION={pkg} (no commit)");
    } else {
        println!("cargo:rustc-env=TYPST_VERSION={pkg} ({hash})");
    }
}

fn typst_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8(output.stdout.strip_suffix(b"\n").unwrap().into()).ok()
        })
        .unwrap_or_else(|| "unknown commit".into())
}
