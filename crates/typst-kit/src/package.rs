//! Download and unpack packages and package indices.

use std::fs;
use std::path::{Path, PathBuf};
use std::{thread::sleep, time::Duration};

use ecow::eco_format;
use once_cell::sync::OnceCell;
use typst::diag::{bail, PackageError, PackageResult, StrResult};
use typst::syntax::package::{
    PackageInfo, PackageSpec, PackageVersion, VersionlessPackageSpec,
};

use crate::download::{Downloader, Progress};
use crate::lockfile::LockFile;

/// The default Typst registry.
pub const DEFAULT_REGISTRY: &str = "https://packages.typst.org";

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
    /// The cached index of the preview namespace.
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

    /// Returns a the path at which non-local packages should be stored when
    /// downloaded.
    pub fn package_cache_path(&self) -> Option<&Path> {
        self.package_cache_path.as_deref()
    }

    /// Returns a the path at which local packages are stored.
    pub fn package_path(&self) -> Option<&Path> {
        self.package_path.as_deref()
    }

    /// Make a package available in the on-disk.
    ///
    /// Will return path to the package from the local package directory if present,
    /// or from the cached directory otherwise.
    pub fn prepare_package(
        &self,
        spec: &PackageSpec,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        let path_to_package =
            format!("{}/{}/{}", spec.namespace, spec.name, spec.version);

        if let Some(packages_dir) = &self.package_path {
            let package_dir = packages_dir.join(&path_to_package);
            if package_dir.exists() {
                return Ok(package_dir);
            }
        }

        if let Some(cache_dir) = &self.package_cache_path {
            let package_dir = cache_dir.join(path_to_package);
            if package_dir.exists() {
                return Ok(package_dir);
            }

            let lock_file = LockFile::create_and_lock(cache_dir, spec)?;
            if lock_file.did_wait() {
                // If other instance successfully installed a package after
                // waiting for the lock, then there is nothing left to do.
                if package_dir.exists() {
                    return Ok(package_dir);
                }

                eprintln!(
                    "Another instance failed to install the package, trying it again."
                );
                // Additional cool/user-friendly logs:
                // let _write_lock = lock_file.write().unwrap();
                // println!("checking if package already exists");
                // if dir.exists() {
                //     println!("it does! no need to download and unpack");
                //     return Ok(dir);
                // } else {
                //     println!("waited to aquire the file lock and the other instance wasn't able to download it :/");
                //     todo!()
                // }
            }

            // Download from network if it doesn't exist yet.
            if spec.namespace == "preview" {
                self.download_package(spec, &package_dir, progress)?;
                if package_dir.exists() {
                    return Ok(package_dir);
                }
            }
        }

        Err(PackageError::NotFound(spec.clone()))
    }

    /// Try to determine the latest version of a package.
    pub fn determine_latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        if spec.namespace == "preview" {
            // For `@preview`, download the package index and find the latest
            // version.
            self.download_index()?
                .iter()
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
    pub fn download_index(&self) -> StrResult<&Vec<PackageInfo>> {
        self.index.get_or_try_init(|| {
            let url = format!("{DEFAULT_REGISTRY}/preview/index.json");
            match self.downloader.download(&url) {
                Ok(response) => response
                    .into_json()
                    .map_err(|err| eco_format!("failed to parse package index: {err}")),
                Err(ureq::Error::Status(404, _)) => {
                    bail!("failed to fetch package index (not found)")
                }
                Err(err) => bail!("failed to fetch package index ({err})"),
            }
        })
    }

    /// Download a package over the network.
    ///
    /// # Panics
    /// Panics if the package spec namespace isn't `preview`.
    pub fn download_package(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()> {
        assert_eq!(spec.namespace, "preview");

        let url =
            format!("{DEFAULT_REGISTRY}/preview/{}-{}.tar.gz", spec.name, spec.version);

        let data = match self.downloader.download_with_progress(&url, progress) {
            Ok(data) => data,
            Err(ureq::Error::Status(404, _)) => {
                if let Ok(version) = self.determine_latest_version(&spec.versionless()) {
                    return Err(PackageError::VersionNotFound(spec.clone(), version));
                } else {
                    return Err(PackageError::NotFound(spec.clone()));
                }
            }
            Err(err) => {
                return Err(PackageError::NetworkFailed(Some(eco_format!("{err}"))))
            }
        };

        eprintln!("[tmp] Unpacking package...");
        sleep(Duration::from_secs(3));
        let decompressed = flate2::read::GzDecoder::new(data.as_slice());
        tar::Archive::new(decompressed).unpack(package_dir).map_err(|err| {
            fs::remove_dir_all(package_dir).ok();
            PackageError::MalformedArchive(Some(eco_format!("{err}")))
        })
    }
}
