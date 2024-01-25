use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use termcolor::WriteColor;
use typst::diag::{PackageError, PackageResult};
use typst::syntax::PackageSpec;

use crate::download::download_with_progress;
use crate::terminal;

/// Make a package available in the on-disk cache.
pub fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
    let subdir =
        format!("typst/packages/{}/{}/{}", spec.namespace, spec.name, spec.version);

    if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join(&subdir);
        if dir.exists() {
            return Ok(dir);
        }
    }

    if let Some(cache_dir) = dirs::cache_dir() {
        let dir = cache_dir.join(&subdir);

        // Download from network if it doesn't exist yet.
        if spec.namespace == "preview" && !dir.exists() {
            download_package(spec, &dir)?;
        }

        if dir.exists() {
            return Ok(dir);
        }
    }

    Err(PackageError::NotFound(spec.clone()))
}

/// Download a package over the network.
fn download_package(spec: &PackageSpec, package_dir: &Path) -> PackageResult<()> {
    // The `@preview` namespace is the only namespace that supports on-demand
    // fetching.
    assert_eq!(spec.namespace, "preview");

    let url = format!(
        "https://packages.typst.org/preview/{}-{}.tar.gz",
        spec.name, spec.version
    );

    print_downloading(spec).unwrap();

    let data = match download_with_progress(&url) {
        Ok(data) => data,
        Err(ureq::Error::Status(404, _)) => {
            return Err(PackageError::NotFound(spec.clone()))
        }
        Err(err) => return Err(PackageError::NetworkFailed(Some(eco_format!("{err}")))),
    };

    let decompressed = flate2::read::GzDecoder::new(data.as_slice());
    tar::Archive::new(decompressed).unpack(package_dir).map_err(|err| {
        fs::remove_dir_all(package_dir).ok();
        PackageError::MalformedArchive(Some(eco_format!("{err}")))
    })
}

/// Print that a package downloading is happening.
fn print_downloading(spec: &PackageSpec) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut term_out = terminal::out();
    term_out.set_color(&styles.header_help)?;
    write!(term_out, "downloading")?;

    term_out.reset()?;
    writeln!(term_out, " {spec}")
}
