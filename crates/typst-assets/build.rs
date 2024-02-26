use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);

    let file_dir = std::fs::canonicalize(manifest_dir.join("files"))?;

    let blob_dir = {
        let dir = option_env!("TYPST_BLOB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| out_dir.join("blobs"));

        // Create the output directory.
        std::fs::create_dir_all(&dir)?;

        // Ensure that the path works no matter the working directory.
        std::fs::canonicalize(dir)?
    };

    println!("cargo:rustc-env=TYPST_FILE_DIR={}", file_dir.display());
    println!("cargo:rustc-env=TYPST_BLOB_DIR={}", blob_dir.display());

    Ok(())
}
