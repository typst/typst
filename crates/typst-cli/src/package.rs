use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use termcolor::WriteColor;
use typst::diag::{bail, PackageError, PackageResult, StrResult};
use typst::syntax::package::{
    PackageInfo, PackageSpec, PackageVersion, VersionlessPackageSpec,
};

use crate::download::{download, download_with_progress};
use crate::terminal;

const HOST: &str = "https://packages.typst.org";

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
        if dir.exists() {
            return Ok(dir);
        }

        // Download from network if it doesn't exist yet.
        if spec.namespace == "preview" {
            download_package(spec, &dir)?;
            if dir.exists() {
                return Ok(dir);
            }
        }
    }

    Err(PackageError::NotFound(spec.clone()))
}

/// Try to determine the latest version of a package.
pub fn determine_latest_version(
    spec: &VersionlessPackageSpec,
) -> StrResult<PackageVersion> {
    if spec.namespace == "preview" {
        // For `@preview`, download the package index and find the latest
        // version.
        download_index()?
            .iter()
            .filter(|package| package.name == spec.name)
            .map(|package| package.version)
            .max()
            .ok_or_else(|| eco_format!("failed to find package {spec}"))
    } else {
        // For other namespaces, search locally. We only search in the data
        // directory and not the cache directory, because the latter is not
        // intended for storage of local packages.
        let subdir = format!("typst/packages/{}/{}", spec.namespace, spec.name);
        dirs::data_dir()
            .into_iter()
            .flat_map(|dir| std::fs::read_dir(dir.join(&subdir)).ok())
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| path.file_name()?.to_string_lossy().parse().ok())
            .max()
            .ok_or_else(|| eco_format!("please specify the desired version"))
    }
}

/// Download a package over the network.
fn download_package(spec: &PackageSpec, package_dir: &Path) -> PackageResult<()> {
    // The `@preview` namespace is the only namespace that supports on-demand
    // fetching.
    assert_eq!(spec.namespace, "preview");

    let url = format!("{HOST}/preview/{}-{}.tar.gz", spec.name, spec.version);

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

/// Download the `@preview` package index.
fn download_index() -> StrResult<Vec<PackageInfo>> {
    let url = format!("{HOST}/preview/index.json");
    match download(&url) {
        Ok(response) => response
            .into_json()
            .map_err(|err| eco_format!("failed to parse package index: {err}")),
        Err(ureq::Error::Status(404, _)) => {
            bail!("failed to fetch package index (not found)")
        }
        Err(err) => bail!("failed to fetch package index ({err})"),
    }
}

/// Print that a package downloading is happening.
fn print_downloading(spec: &PackageSpec) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut out = terminal::out();
    out.set_color(&styles.header_help)?;
    write!(out, "downloading")?;

    out.reset()?;
    writeln!(out, " {spec}")
}
