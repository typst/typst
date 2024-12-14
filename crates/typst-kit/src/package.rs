//! Download and unpack packages and package indices.
use std::path::{Path, PathBuf};

use crate::package_downloads::{Downloader, PackageDownloader, Progress};
use ecow::eco_format;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use typst_library::diag::{bail, PackageError, PackageResult, StrResult};
use typst_syntax::package::{PackageSpec, PackageVersion, VersionlessPackageSpec};

/// The default packages sub directory within the package and package cache paths.
pub const DEFAULT_PACKAGES_SUBDIR: &str = "typst/packages";

/// The default vendor sub directory within the project root.
pub const DEFAULT_VENDOR_SUBDIR: &str = "vendor";

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
    index: OnceCell<Vec<serde_json::Value>>,
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
        Self::with_index(package_cache_path, package_path, downloader, OnceCell::new())
    }

    /// Creates a new package storage with a pre-defined index.
    ///
    /// Useful for testing.
    fn with_index(
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
        downloader: Downloader,
        index: OnceCell<Vec<serde_json::Value>>,
    ) -> Self {
        Self {
            package_vendor_path: package_vendor_path
                .or_else(|| workdir.map(|workdir| workdir.join(DEFAULT_VENDOR_SUBDIR))),
            package_cache_path: package_cache_path.or_else(|| {
                dirs::cache_dir().map(|cache_dir| cache_dir.join(DEFAULT_PACKAGES_SUBDIR))
            }),
            package_path: package_path.or_else(|| {
                dirs::data_dir().map(|data_dir| data_dir.join(DEFAULT_PACKAGES_SUBDIR))
            }),
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

    /// Make a package available in the on-disk.
    pub fn prepare_package(
        &self,
        spec: &PackageSpec,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        let subdir = format!("{}/{}/{}", spec.namespace, spec.name, spec.version);

        // Read from vendor dir if it exists.
        if let Some(vendor_dir) = &self.package_vendor_path {
            if let Ok(true) = vendor_dir.try_exists() {
                let dir = vendor_dir.join(&subdir);
                if dir.exists() {
                    return Ok(dir);
                }
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

    /// Try to determine the latest version of a package.
    pub fn determine_latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        if spec.namespace == DEFAULT_NAMESPACE {
            // For `DEFAULT_NAMESPACE`, download the package index and find the latest
            // version.
            self.download_index()?
                .iter()
                .filter_map(|value| MinimalPackageInfo::deserialize(value).ok())
                .filter(|package| package.name == spec.name)
                .map(|package| package.version)
                .max()
                .ok_or_else(|| eco_format!("failed to find package {spec}"))
        } else {
            // For other namespaces, search locally. We only search in the data
            // directory and not the cache directory, because the latter is not
            // intended for storage of local packages.
            let subdir = format!("{}/{}", spec.namespace, spec.name);
            self.package_path
                .iter()
                .flat_map(|dir| std::fs::read_dir(dir.join(&subdir)).ok())
                .flatten()
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter_map(|path| path.file_name()?.to_string_lossy().parse().ok())
                .max()
                .ok_or_else(|| eco_format!("please specify the desired version"))
        }

        self.download_index(spec)?
            .iter()
            .filter(|package| package.name == spec.name)
            .map(|package| package.version)
            .max()
            .ok_or_else(|| eco_format!("failed to find package {spec}"))
    }

    /// Download the package index. The result of this is cached for efficiency.
    pub fn download_index(&self) -> StrResult<&[serde_json::Value]> {
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

/// Minimal information required about a package to determine its latest
/// version.
#[derive(Deserialize)]
struct MinimalPackageInfo {
    name: String,
    version: PackageVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_deser_index() {
        let storage = PackageStorage::with_index(
            None,
            None,
            Downloader::new("typst/test"),
            OnceCell::with_value(vec![
                serde_json::json!({
                    "name": "charged-ieee",
                    "version": "0.1.0",
                    "entrypoint": "lib.typ",
                }),
                serde_json::json!({
                    "name": "unequivocal-ams",
                    // This version number is currently not valid, so this package
                    // can't be parsed.
                    "version": "0.2.0-dev",
                    "entrypoint": "lib.typ",
                }),
            ]),
        );

        let ieee_version = storage.determine_latest_version(&VersionlessPackageSpec {
            namespace: "preview".into(),
            name: "charged-ieee".into(),
        });
        assert_eq!(ieee_version, Ok(PackageVersion { major: 0, minor: 1, patch: 0 }));

        let ams_version = storage.determine_latest_version(&VersionlessPackageSpec {
            namespace: "preview".into(),
            name: "unequivocal-ams".into(),
        });
        assert_eq!(
            ams_version,
            Err("failed to find package @preview/unequivocal-ams".into())
        )
    }
}
