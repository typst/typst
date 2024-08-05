use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::{thread::sleep, time::Duration};

use crate::args::PackageStorageArgs;
use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use fd_lock::RwLock;
use once_cell::sync::OnceCell;
use termcolor::WriteColor;
use typst::diag::{bail, PackageError, PackageResult, StrResult};
use typst::syntax::package::{
    PackageInfo, PackageSpec, PackageVersion, VersionlessPackageSpec,
};

use crate::download::{download, download_with_progress};
use crate::terminal;

const HOST: &str = "https://packages.typst.org";
const DEFAULT_PACKAGES_SUBDIR: &str = "typst/packages";

/// Holds information about where packages should be stored.
pub struct PackageStorage {
    pub package_cache_path: Option<PathBuf>,
    pub package_path: Option<PathBuf>,
    index: OnceCell<Vec<PackageInfo>>,
}

impl PackageStorage {
    pub fn from_args(args: &PackageStorageArgs) -> Self {
        let package_cache_path = args.package_cache_path.clone().or_else(|| {
            dirs::cache_dir().map(|cache_dir| cache_dir.join(DEFAULT_PACKAGES_SUBDIR))
        });
        let package_path = args.package_path.clone().or_else(|| {
            dirs::data_dir().map(|data_dir| data_dir.join(DEFAULT_PACKAGES_SUBDIR))
        });
        Self {
            package_cache_path,
            package_path,
            index: OnceCell::new(),
        }
    }

    /// Make a package available in the on-disk cache.
    pub fn prepare_package(&self, spec: &PackageSpec) -> PackageResult<PathBuf> {
        let deepest_dir = spec.version;
        let subdir = format!("{}/{}/{}", spec.namespace, spec.name, deepest_dir);

        if let Some(packages_dir) = &self.package_path {
            let dir = packages_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
        }

        if let Some(cache_dir) = &self.package_cache_path {
            let dir = cache_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
            let parent_dir = dir.parent().unwrap();

            let file_locking_err = |string| {
                move |err| PackageError::FileLocking(eco_format!("{string}: {err}"))
            };

            let lock_file_path = parent_dir.join(format!(".{deepest_dir}.lock"));
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
            let mut lock_file = RwLock::new(file);
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
                self.download_package(spec, &dir)?;
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
}

impl PackageStorage {
    /// Download a package over the network.
    fn download_package(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
    ) -> PackageResult<()> {
        // The `@preview` namespace is the only namespace that supports on-demand
        // fetching.
        assert_eq!(spec.namespace, "preview");

        let url = format!("{HOST}/preview/{}-{}.tar.gz", spec.name, spec.version);

        print_downloading(spec).unwrap();

        let data = match download_with_progress(&url) {
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

        eprintln!("[tmp] unpacking the archive...");
        sleep(Duration::from_secs(5));
        let decompressed = flate2::read::GzDecoder::new(data.as_slice());
        tar::Archive::new(decompressed).unpack(package_dir).map_err(|err| {
            fs::remove_dir_all(package_dir).ok();
            PackageError::MalformedArchive(Some(eco_format!("{err}")))
        })
    }

    /// Download the `@preview` package index.
    ///
    /// To avoid downloading the index multiple times, the result is cached.
    fn download_index(&self) -> StrResult<&Vec<PackageInfo>> {
        self.index.get_or_try_init(|| {
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
        })
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
