//! Download and unpack packages and package indices.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ecow::eco_format;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use typst_library::diag::{
    PackageError, PackageRegistry, PackageResult, StrResult, bail,
};
use typst_syntax::package::{PackageSpec, PackageVersion, VersionlessPackageSpec};

use crate::download::{Downloader, Progress};

/// The default Typst registry.
pub const DEFAULT_REGISTRY: &str = "https://packages.typst.org";

/// The public namespace in the default Typst registry.
pub const DEFAULT_NAMESPACE: &str = "preview";

/// The default packages sub directory within the package and package cache paths.
pub const DEFAULT_PACKAGES_SUBDIR: &str = "typst/packages";

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
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
        downloader: Downloader,
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

        // By default, search for the package locally
        if let Some(packages_dir) = &self.package_path {
            let dir = packages_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
        }

        // As a fallback, look into the cache and possibly download from network.
        if let Some(cache_dir) = &self.package_cache_path {
            let dir = cache_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }

            // Download from network if it doesn't exist yet.
            if spec.namespace == DEFAULT_NAMESPACE {
                return self.download_package(spec, cache_dir, progress);
            }
        }

        // None of the strategies above found the package, so all code paths
        // from now on fail. The rest of the function is only to determine the
        // cause of the failure.
        // We try `namespace/` then `namespace/name/` then `namespace/name/version/`
        // and see where the first error occurs.
        let not_found = |msg| Err(Box::new(PackageError::NotFound(spec.clone(), msg)));

        let Some(packages_dir) = &self.package_path else {
            return not_found(eco_format!("cannot access local package storage"));
        };
        let namespace_dir = packages_dir.join(format!("{}", spec.namespace));
        if !namespace_dir.exists() {
            return not_found(eco_format!(
                "the namespace @{} should be located at {}",
                spec.namespace,
                namespace_dir.display()
            ));
        }
        let package_dir = namespace_dir.join(format!("{}", spec.name));
        if !package_dir.exists() {
            return not_found(eco_format!(
                "the registry at {} does not have package '{}'",
                namespace_dir.display(),
                spec.name
            ));
        }
        let latest = self.determine_latest_version(&spec.versionless()).ok();
        Err(Box::new(PackageError::VersionNotFound(
            spec.clone(),
            latest,
            PackageRegistry {
                name: eco_format!("{}", spec.namespace),
                path: namespace_dir.to_path_buf(),
                url: None,
            },
        )))
    }

    /// Tries to determine the latest version of a package.
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
    }

    /// Download the package index. The result of this is cached for efficiency.
    fn download_index(&self) -> StrResult<&[serde_json::Value]> {
        self.index
            .get_or_try_init(|| {
                let url = format!("{DEFAULT_REGISTRY}/{DEFAULT_NAMESPACE}/index.json");
                match self.downloader.download(&url) {
                    Ok(response) => response.into_json().map_err(|err| {
                        eco_format!("failed to parse package index: {err}")
                    }),
                    Err(ureq::Error::Status(404, _)) => {
                        bail!("failed to fetch package index (not found)")
                    }
                    Err(err) => bail!("failed to fetch package index ({err})"),
                }
            })
            .map(AsRef::as_ref)
    }

    /// Download a package over the network.
    ///
    /// # Panics
    /// Panics if the package spec namespace isn't `DEFAULT_NAMESPACE`.
    fn download_package(
        &self,
        spec: &PackageSpec,
        cache_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        assert_eq!(spec.namespace, DEFAULT_NAMESPACE);

        let namespace_url = format!("{DEFAULT_REGISTRY}/{DEFAULT_NAMESPACE}");
        let url = format!("{namespace_url}/{}-{}.tar.gz", spec.name, spec.version);

        let data = match self.downloader.download_with_progress(&url, progress) {
            Ok(data) => data,
            Err(ureq::Error::Status(404, _)) => {
                if let Ok(version) = self.determine_latest_version(&spec.versionless()) {
                    return Err(Box::new(PackageError::VersionNotFound(
                        spec.clone(),
                        Some(version),
                        PackageRegistry {
                            name: eco_format!("{DEFAULT_REGISTRY}"),
                            path: cache_dir.to_path_buf(),
                            url: Some(eco_format!("{namespace_url}")),
                        },
                    )));
                } else {
                    return Err(Box::new(PackageError::NotFound(
                        spec.clone(),
                        eco_format!(
                            "the registry at {namespace_url} does not have package '{}'",
                            spec.name
                        ),
                    )));
                }
            }
            Err(err) => {
                return Err(Box::new(PackageError::NetworkFailed(Some(eco_format!(
                    "{err}"
                )))));
            }
        };

        // The directory in which the package's version lives.
        let base_dir = cache_dir.join(format!("{}/{}", spec.namespace, spec.name));

        // The place at which the specific package version will live in the end.
        let package_dir = base_dir.join(format!("{}", spec.version));

        // To prevent multiple Typst instances from interfering, we download
        // into a temporary directory first and then move this directory to
        // its final destination.
        //
        // In the `rename` function's documentation it is stated:
        // > This will not work if the new name is on a different mount point.
        //
        // By locating the temporary directory directly next to where the
        // package directory will live, we are (trying our best) making sure
        // that `tempdir` and `package_dir` are on the same mount point.
        let tempdir = Tempdir::create(base_dir.join(format!(
            ".tmp-{}-{}",
            spec.version,
            fastrand::u32(..),
        )))
        .map_err(|err| error("failed to create temporary package directory", err))?;

        // Decompress the archive into the temporary directory.
        let decompressed = flate2::read::GzDecoder::new(data.as_slice());
        tar::Archive::new(decompressed)
            .unpack(&tempdir)
            .map_err(|err| PackageError::MalformedArchive(Some(eco_format!("{err}"))))?;

        // When trying to move (i.e., `rename`) the directory from one place to
        // another and the target/destination directory is empty, then the
        // operation will succeed (if it's atomic, or hardware doesn't fail, or
        // power doesn't go off, etc.). If however the target directory is not
        // empty, i.e., another instance already successfully moved the package,
        // then we can safely ignore the `DirectoryNotEmpty` error.
        //
        // This means that we do not check the integrity of an existing moved
        // package, just like we don't check the integrity if the package
        // directory already existed in the first place. If situations with
        // broken packages still occur even with the rename safeguard, we might
        // consider more complex solutions like file locking or checksums.
        match fs::rename(&tempdir, &package_dir) {
            Ok(()) => Ok(package_dir),
            Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(package_dir),
            Err(err) => {
                Err(Box::new(error("failed to move downloaded package directory", err)))
            }
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

/// A temporary directory that is a automatically cleaned up.
struct Tempdir(PathBuf);

impl Tempdir {
    /// Creates a directory at the path and auto-cleans it.
    fn create(path: PathBuf) -> io::Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}

impl Drop for Tempdir {
    fn drop(&mut self) {
        _ = fs::remove_dir_all(&self.0);
    }
}

impl AsRef<Path> for Tempdir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Enriches an I/O error with a message and turns it into a
/// `PackageError::Other`.
#[cold]
fn error(message: &str, err: io::Error) -> PackageError {
    PackageError::Other(Some(eco_format!("{message}: {err}")))
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
