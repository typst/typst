//! Download and unpack packages and package indices.

use std::path::{Path, PathBuf};

use crate::package_downloads::{Downloader, PackageDownloader, Progress};
use ecow::eco_format;
use once_cell::sync::OnceCell;
use typst_library::diag::{PackageError, PackageResult, StrResult};
use typst_syntax::package::{
    PackageInfo, PackageSpec, PackageVersion, VersionlessPackageSpec,
};

/// The default packages sub directory within the package and package cache paths.
pub const DEFAULT_PACKAGES_SUBDIR: &str = "typst/packages";

/// The default vendor sub directory within the project root.
pub const DEFAULT_VENDOR_SUBDIR: &str = "vendor";

/// Attempts to infer the default package cache directory from the current
/// environment.
///
/// This simply joins [`DEFAULT_PACKAGES_SUBDIR`] to the output of
/// [`dirs::cache_dir`].
pub fn default_package_cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|cache_dir| cache_dir.join(DEFAULT_PACKAGES_SUBDIR))
}

/// Attempts to infer the default package directory from the current
/// environment.
///
/// This simply joins [`DEFAULT_PACKAGES_SUBDIR`] to the output of
/// [`dirs::data_dir`].
pub fn default_package_path() -> Option<PathBuf> {
    dirs::data_dir().map(|data_dir| data_dir.join(DEFAULT_PACKAGES_SUBDIR))
}

/// Holds information about where packages should be stored and downloads them
/// on demand, if possible.
#[derive(Debug)]
pub struct PackageStorage {
    /// The path at which packages are stored by the vendor command.
    package_vendor_path: Option<PathBuf>,
    /// The path at which non-local packages should be stored when downloaded.
    package_cache_path: Option<PathBuf>,
    /// The path at which local packages are stored.
    package_path: Option<PathBuf>,
    /// The downloader used for fetching the index and packages.
    downloader: Downloader,
    /// The cached index of the default namespace.
    index: OnceCell<Vec<PackageInfo>>,
}

impl PackageStorage {
    /// Creates a new package storage for the given package paths. Falls back to
    /// the recommended XDG directories if they are `None`.
    pub fn new(
        package_vendor_path: Option<PathBuf>,
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
        downloader: Downloader,
        workdir: Option<PathBuf>,
    ) -> Self {
        Self::with_index(
            package_vendor_path,
            workdir,
            package_cache_path,
            package_path,
            downloader,
            OnceCell::new(),
        )
    }

    /// Creates a new package storage with a pre-defined index.
    ///
    /// Useful for testing.
    fn with_index(
        package_vendor_path: Option<PathBuf>,
        workdir: Option<PathBuf>,
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
        downloader: Downloader,
        index: OnceCell<Vec<PackageInfo>>,
    ) -> Self {
        Self {
            package_vendor_path: package_vendor_path
                .or_else(|| workdir.map(|workdir| workdir.join(DEFAULT_VENDOR_SUBDIR))),
            package_cache_path: package_cache_path.or_else(default_package_cache_path),
            package_path: package_path.or_else(default_package_path),
            downloader,
            index,
        }
    }

    /// Returns the path at which non-local packages should be stored when
    /// downloaded.
    pub fn package_cache_path(&self) -> Option<&Path> {
        self.package_cache_path.as_deref()
    }

    /// Returns the path at which local packages are stored.
    pub fn package_path(&self) -> Option<&Path> {
        self.package_path.as_deref()
    }

    /// Makes a package available on-disk and returns the path at which it is
    /// located (will be either in the cache or package directory).
    pub fn prepare_package(
        &self,
        spec: &PackageSpec,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        let subdir = format!("{}/{}/{}", spec.namespace, spec.name, spec.version);

        // Read from vendor dir if it exists.
        if let Some(vendor_dir) = &self.package_vendor_path
            && let Ok(true) = vendor_dir.try_exists()
        {
            let dir = vendor_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
        }

        if let Some(packages_dir) = &self.package_path {
            let dir = packages_dir.join(&subdir);
            if dir.exists() {
                // no need to download, already in the path.
                return Ok(dir);
            }
        }

        // package was not in the package_path. check if it has been cached
        if let Some(cache_dir) = &self.package_cache_path {
            let dir = cache_dir.join(&subdir);
            if dir.exists() {
                //package was cached, so return the cached directory
                return Ok(dir);
            }

            // Download from network if it doesn't exist yet.
            self.download_package(spec, &dir, progress)?;
            if dir.exists() {
                return Ok(dir);
            }
        }

        Err(PackageError::NotFound(spec.clone()))
    }

    /// Tries to determine the latest version of a package.
    pub fn determine_latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        self.download_index(spec)?
            .iter()
            .filter(|package| package.name == spec.name)
            .map(|package| package.version)
            .max()
            .ok_or_else(|| eco_format!("failed to find package {spec}"))
    }

    /// Download the package index. The result of this is cached for efficiency.
    pub fn download_index(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<&[PackageInfo]> {
        self.index
            .get_or_try_init(|| self.downloader.download_index(spec))
            .map(AsRef::as_ref)
    }

    /// Download a package over the network.
    ///
    /// # Panics
    /// Panics if the package spec namespace isn't `DEFAULT_NAMESPACE`.
    fn download_package(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()> {
        match self.downloader.download(spec, package_dir, progress) {
            Err(PackageError::NotFound(spec)) => {
                if let Ok(version) = self.determine_latest_version(&spec.versionless()) {
                    Err(PackageError::VersionNotFound(spec.clone(), version))
                } else {
                    Err(PackageError::NotFound(spec.clone()))
                }
            }
            val => val,
        }
    }
}

// /// Minimal information required about a package to determine its latest
// /// version.
// #[derive(Deserialize)]
// struct MinimalPackageInfo {
//     name: String,
//     version: PackageVersion,
// }

// /// A temporary directory that is a automatically cleaned up.
// struct Tempdir(PathBuf);

// impl Tempdir {
//     /// Creates a directory at the path and auto-cleans it.
//     fn create(path: PathBuf) -> io::Result<Self> {
//         std::fs::create_dir_all(&path)?;
//         Ok(Self(path))
//     }
// }

// impl Drop for Tempdir {
//     fn drop(&mut self) {
//         _ = fs::remove_dir_all(&self.0);
//     }
// }

// impl AsRef<Path> for Tempdir {
//     fn as_ref(&self) -> &Path {
//         &self.0
//     }
// }

// /// Enriches an I/O error with a message and turns it into a
// /// `PackageError::Other`.
// #[cold]
// fn error(message: &str, err: io::Error) -> PackageError {
//     PackageError::Other(Some(eco_format!("{message}: {err}")))
// }
