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

/// Holds information about where packages should be stored and downloads them
/// on demand, if possible.
#[derive(Debug)]
pub struct PackageStorage {
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
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
        downloader: Downloader,
    ) -> Self {
        Self {
            package_cache_path: package_cache_path.or_else(|| {
                dirs::cache_dir().map(|cache_dir| cache_dir.join(DEFAULT_PACKAGES_SUBDIR))
            }),
            package_path: package_path.or_else(|| {
                dirs::data_dir().map(|data_dir| data_dir.join(DEFAULT_PACKAGES_SUBDIR))
            }),
            downloader,
            index: OnceCell::new(),
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

    /// Make a package available in the on-disk.
    pub fn prepare_package(
        &self,
        spec: &PackageSpec,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        let subdir = format!("{}/{}/{}", spec.namespace, spec.name, spec.version);

        // check the package_path for the package directory.
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

    /// Try to determine the latest version of a package.
    pub fn determine_latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        // Same logical flow as per package download. Check package path, then check online.
        // Do not check in the data directory because the latter is not intended for storage
        // of local packages.
        let subdir = format!("{}/{}", spec.namespace, spec.name);
        let res = self
            .package_path
            .iter()
            .flat_map(|dir| std::fs::read_dir(dir.join(&subdir)).ok())
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| path.file_name()?.to_string_lossy().parse().ok())
            .max();

        if let Some(version) = res {
            return Ok(version);
        }

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
    pub fn download_package(
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
