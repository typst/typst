//! Download and unpack packages and package indices.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::{thread::sleep, time::Duration};

use ecow::eco_format;
use once_cell::sync::OnceCell;
use typst::diag::{bail, PackageError, PackageResult, StrResult};
use typst::syntax::package::{
    PackageInfo, PackageSpec, PackageVersion, VersionlessPackageSpec,
};

use crate::download::{Downloader, Progress};

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

/// Returns mapped `io::Error` to `PackageError::FileLocking` with a custom message.
/// Intended to be used inside `io::Result`'s `.map_err()` method.
fn file_locking_err(message: &'static str) -> impl Fn(io::Error) -> PackageError {
    move |error| PackageError::FileLocking(eco_format!("{message}: {error}"))
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
    pub fn prepare_package(
        &self,
        spec: &PackageSpec,
        progress: &mut dyn Progress,
    ) -> PackageResult<PathBuf> {
        let parent_subdir = format!("{}/{}", spec.namespace, spec.name);
        let subdir = format!("{parent_subdir}/{}", spec.version);

        if let Some(packages_dir) = &self.package_path {
            let dir = packages_dir.join(&subdir);
            if dir.exists() {
                // Question:
                // This is a dir in the persistent package dir.
                // Which dir is supposed to be returned?
                return Ok(dir);
            }
        }

        if let Some(cache_dir) = &self.package_cache_path {
            let dir = cache_dir.join(subdir);
            if dir.exists() {
                // Question:
                // This is a dir in the temporary cache dir.
                // Which dir is supposed to be returned?
                return Ok(dir);
            }
            let parent_dir = cache_dir.join(&parent_subdir);

            let lock_file_path = parent_dir.join(format!(".{}.lock", spec.version));
            let remove_lock = || {
                fs::remove_file(&lock_file_path)
                    .map_err(file_locking_err("failed to remove lock file"))
            };
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)
                    .map_err(file_locking_err("failed to create parent directories"))?;
            }
            // https://github.com/yoshuawuyts/fd-lock/issues/28#issuecomment-2264180893
            // File::create(&lock_file_path).unwrap();
            // let file = match File::open(lock_file_path.as_path()) {
            let file = fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                // .read(true) // to test the error
                .open(lock_file_path.as_path())
                .map_err(file_locking_err("failed to open lock file"))?;
            let lock_file_path = format!(
                "~{}",
                lock_file_path.to_string_lossy().strip_prefix(env!("HOME")).unwrap()
            );
            // let mut lock_file = RwLock::new(file.try_clone().unwrap());
            let mut lock_file = fd_lock::RwLock::new(file);
            let try_lock = lock_file.try_write();
            let _lock = match try_lock {
                Ok(lock) => {
                    eprintln!("[tmp] acquired the lock file {lock_file_path:?}");
                    lock
                }
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    drop(try_lock);
                    eprintln!("[tmp] couldn't aquire the lock file {lock_file_path:?}");
                    eprintln!("Waiting for another instance to finish installing the package...");
                    let lock = lock_file
                        .write()
                        .map_err(file_locking_err("failed to aquire lock file"))?;

                    // If other instance successfully installed a package after
                    // waiting for the lock, then there is nothing left to do.
                    if dir.exists() {
                        eprintln!("[tmp] dropped the lock file {lock_file_path:?}");
                        return Ok(dir);
                    }

                    eprintln!("Another instance failed to install the package, trying it again.");
                    lock
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
                Err(err) => {
                    return Err(file_locking_err("failed to aquire lock file")(err))
                }
            };

            // Download from network if it doesn't exist yet.
            if spec.namespace == "preview" {
                self.download_package(spec, &dir, progress)?;
                // https://github.com/yoshuawuyts/fd-lock/issues/28#issuecomment-2264180893
                // drop(lock);
                // remove_file(&lock_file_path).unwrap();
                remove_lock()?;
                eprintln!("[tmp] dropped the lock file {lock_file_path:?}");
                if dir.exists() {
                    return Ok(dir);
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
