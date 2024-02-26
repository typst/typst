/*!
Typst's build process and tests depend on assets (mostly fonts, but also other
resources like images, text files, etc.). Some smaller ones are stored directly
in this crate's `files/` directory. However, others are large and we don't want
to store them within the Git repository. Those are downloaded on-demand from
`TYPST_BLOB_URL` (defaults to `https://gitblobs.typst.org`) with the help of
this crate.

The build process will take care of downloading the blobs automatically.
However, if you want to set up an environment suitable for offline building, you
can also download them upfront by running:

```bash
cargo run -p typst-assets
```

This will place the blobs into `TYPST_BLOB_DIR`, which defaults to a directory
within a Rust-provided `OUT_DIR`. The path specified in the environment
variable will be relative the `crates/typst-assets` directory.
*/

mod blobs;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use native_tls::TlsConnector;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

use self::blobs::BLOBS;

/// The url from which the blobs are downloaded.
const BLOB_URL: &str = match option_env!("TYPST_BLOB_URL") {
    Some(url) => url,
    None => "https://gitblobs.typst.org",
};

/// The directory where smaller bundled files are.
const FILE_DIR: &str = env!("TYPST_FILE_DIR");

/// The directory where large blobs are. Those will only be downloaded
/// on-demand.
const BLOB_DIR: &str = env!("TYPST_BLOB_DIR");

/// Makes an asset available and then immediately loads it.
pub fn get(filename: &str) -> anyhow::Result<Vec<u8>> {
    let path = path(filename)?;
    Ok(std::fs::read(path)?)
}

/// Makes an asset available and returns the path of it.
///
/// The path is only valid on the host system where `typst-assets` was compiled.
/// Assets that should be baked into the final binary must use the
/// `include_asset` macro from `typst_macros`.
///
/// - When this is a file in `files/`, it is accessed directly.
/// - When it is a well-known blob, it is downloaded.
/// - A blob that is already present is not downloaded again.
/// - Creates the blob directory recursively if it doesn't exist.
/// - If the download or I/O operations fail, returns an error.
pub fn path(filename: &str) -> anyhow::Result<PathBuf> {
    // First try to resolve as a vendored assets.
    let file_path = Path::new(FILE_DIR).join(filename);
    if file_path.exists() {
        return Ok(file_path);
    }

    // Then, try to find a blob.
    let index = BLOBS
        .binary_search_by_key(&filename, |(name, _)| name)
        .map_err(|_| anyhow::anyhow!("asset `{filename}` is not known"))?;

    let blob_dir = Path::new(BLOB_DIR);
    let hash = BLOBS[index].1;
    let dest = blob_dir.join(hash);
    if dest.exists() {
        return Ok(dest);
    }

    print_downloading(filename).ok();

    std::fs::create_dir_all(blob_dir).context("failed to create blob directory")?;

    let agent = ureq::AgentBuilder::new()
        .user_agent(concat!("typst-assets/", env!("CARGO_PKG_VERSION")))
        .tls_connector(Arc::new(
            TlsConnector::new().context("failed to build tls connector")?,
        ))
        .build();

    let mut data = vec![];
    agent
        .get(&format!("{BLOB_URL}/{hash}"))
        .call()
        .with_context(|| format!("failed to fetch asset `{filename}`"))?
        .into_reader()
        .read_to_end(&mut data)
        .with_context(|| format!("failed to download asset `{filename}`"))?;
    std::fs::write(&dest, data).context("failed to write asset to disk")?;

    Ok(dest)
}

/// Provides all font blobs.
pub fn fonts() -> impl Iterator<Item = anyhow::Result<Vec<u8>>> {
    BLOBS
        .par_iter()
        .map(|&(filename, _)| filename)
        .filter(|filename| filename.ends_with("ttf") || filename.ends_with("otf"))
        .map(get)
        .collect::<Vec<_>>()
        .into_iter()
}

/// Downloads all blobs for offline use.
pub fn download_all() -> anyhow::Result<()> {
    BLOBS
        .par_iter()
        .try_for_each(|(filename, _)| path(filename).map(|_| ()))
}

/// Print that we are downloading a blob.
fn print_downloading(filename: &str) -> io::Result<()> {
    let stream = StandardStream::stderr(ColorChoice::Auto);
    let mut locked = stream.lock();
    let color = ColorSpec::new()
        .set_bold(true)
        .set_fg(Some(termcolor::Color::Green))
        .clone();
    write!(locked, " ")?;
    locked.set_color(&color)?;
    write!(locked, "Downloading")?;
    locked.reset()?;
    writeln!(locked, " {filename} ({BLOB_URL})")
}
