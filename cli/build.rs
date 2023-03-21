use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    let output = Command::new("git").args(&["rev-parse", "HEAD"]).output()?;
    let hash = std::str::from_utf8(&output.stdout)?;
    println!("cargo:rustc-env=TYPST_HASH={}", &hash[..8]);
    Ok(())
}
