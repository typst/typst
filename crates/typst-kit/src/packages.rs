//! Package loading.

use std::path::{Path, PathBuf};

use ecow::eco_format;
use typst_syntax::package::{PackageSpec, PackageVersion, VersionlessPackageSpec};

use crate::files::FsRoot;

#[cfg(feature = "universe-packages")]
use {
    crate::downloader::Downloader,
    once_cell::sync::OnceCell,
    serde::Deserialize,
    std::io::{Cursor, Read},
    typst_library::diag::{PackageError, PackageResult, StrResult, bail},
};

/// Serves packages from standard locations.
///
/// In order of priority, this tries to obtain a package from
///
/// - a package data directory (that is intended for system-wide storage of user
///   packages)
/// - a package cache directory (that is intended for caching of automatically
///   downloaded packages)
/// - by downloading it from Typst Universe or a mirror of it (if it's namespace
///   matches the one Typst Universe serves)
///
/// With default configuration, this loads packages from the same sources as the
/// CLI.
#[cfg(feature = "system-packages")]
pub struct SystemPackages {
    data: Option<FsPackages>,
    cache: Option<FsPackages>,
    universe: UniversePackages,
}

#[cfg(feature = "system-packages")]
impl SystemPackages {
    /// Creates a new handle that serves packages from standard
    /// environment-defined directories and the official Typst Universe
    /// registry.
    ///
    /// - See [`FsPackages`] for more details on the default directories.
    /// - See [`UniversePackages`] for more details on the registry.
    ///
    /// This loads packages from the same sources as the CLI in its default
    /// configuration.
    pub fn new(downloader: impl Downloader) -> Self {
        Self::from_parts(
            FsPackages::system_data(),
            FsPackages::system_cache(),
            UniversePackages::new(downloader),
        )
    }

    /// Creates a new system package loader from custom configured parts.
    pub fn from_parts(
        data: Option<FsPackages>,
        cache: Option<FsPackages>,
        universe: UniversePackages,
    ) -> Self {
        Self { data, cache, universe }
    }

    /// Returns a handle to the data package directory.
    pub fn data(&self) -> Option<&FsPackages> {
        self.data.as_ref()
    }

    /// Returns a handle to the cache package directory.
    pub fn cache(&self) -> Option<&FsPackages> {
        self.cache.as_ref()
    }

    /// Returns a handle to the Typst universe registry.
    pub fn universe(&self) -> &UniversePackages {
        &self.universe
    }

    /// Returns the file system root from which the given package's content can
    /// be loaded.
    ///
    /// May download the package from the network if it's not already available.
    /// Downloads are retained in the configured cache directory. As such, this
    /// function can have a file system side effect.
    ///
    /// Concurrent downloads do not cause corruption, but for the purpose of
    /// efficiency, it may be desirable to avoid them. If you use the
    /// [`FileStore`](crate::files::FileStore), this is already the case since
    /// it acquires a lock during file loading.
    pub fn obtain(&self, spec: &PackageSpec) -> PackageResult<FsRoot> {
        if let Some(packages) = &self.data
            && let Some(root) = packages.obtain(spec)
        {
            return Ok(root);
        }

        if let Some(cache) = &self.cache {
            if let Some(root) = cache.obtain(spec) {
                return Ok(root);
            }

            // Download from network if it doesn't exist yet.
            if spec.namespace == UniversePackages::NAMESPACE {
                let mut archive = self.universe.package(spec)?;

                cache.store(spec, |tempdir| {
                    archive.unpack(tempdir).map_err(|err| {
                        PackageError::MalformedArchive(Some(eco_format!("{err}")))
                    })
                })?;

                if let Some(root) = cache.obtain(spec) {
                    return Ok(root);
                }
            }
        }

        Err(PackageError::NotFound(spec.clone()))
    }

    /// Tries to determine the latest version of a package.
    pub fn latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        if spec.namespace == UniversePackages::NAMESPACE {
            self.universe.latest_version(spec)
        } else {
            // For other namespaces, search locally. We only search in the data
            // directory and not the cache directory, because the latter is not
            // intended for storage of local packages.
            self.data
                .as_ref()
                .and_then(|pkgs| pkgs.latest_version(spec))
                .ok_or_else(|| eco_format!("please specify the desired version"))
        }
    }
}

/// Serves packages from a well-structured directory on the file system.
///
/// This directory should be structured as follows:
/// - Top-level directories denote namespaces
/// - Second-level directories denote packages
/// - Third-level directories denote package versions
pub struct FsPackages(PathBuf);

impl FsPackages {
    /// Creates a new handle that serves packages from the given directory.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Tries to provide a handle to the environment-defined standard system
    /// package data directory.
    ///
    /// This is:
    /// - `$XDG_DATA_HOME/typst/packages` or `~/.local/share/typst/packages` on Linux
    /// - `~/Library/Application Support/typst/packages` on macOS
    /// - `%APPDATA%/typst/packages` on Windows
    #[cfg(feature = "system-packages")]
    pub fn system_data() -> Option<FsPackages> {
        dirs::data_dir().map(|dir| FsPackages::new(dir.join("typst/packages")))
    }

    /// Tries to provide a handle to the environment-defined standard system
    /// package cache directory.
    ///
    /// This is:
    /// - `$XDG_CACHE_HOME/typst/packages` or `~/.cache/typst/packages` on Linux
    /// - `~/Library/Caches/typst/packages` on macOS
    /// - `%LOCALAPPDATA%/typst/packages` on Windows
    #[cfg(feature = "system-packages")]
    pub fn system_cache() -> Option<FsPackages> {
        dirs::cache_dir().map(|dir| FsPackages::new(dir.join("typst/packages")))
    }

    /// Returns the path from which this serves packages.
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Returns the file system root from which the given package's content can
    /// be loaded.
    pub fn obtain(&self, spec: &PackageSpec) -> Option<FsRoot> {
        let subdir = eco_format!("{}/{}/{}", spec.namespace, spec.name, spec.version);
        let dir = self.path().join(subdir.as_str());
        dir.exists().then_some(FsRoot::new(dir))
    }

    /// Tries to determine the latest version of a particular package in the
    /// directory tree.
    pub fn latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> Option<PackageVersion> {
        // For other namespaces, search locally. We only search in the data
        // directory and not the cache directory, because the latter is not
        // intended for storage of local packages.
        let subdir = format!("{}/{}", spec.namespace, spec.name);
        std::fs::read_dir(self.path().join(&subdir))
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| path.file_name()?.to_string_lossy().parse().ok())
            .max()
    }

    /// Stores data for the given package in the package directory by invoking
    /// `write` on a target path. The package contents should be written
    /// relative to the path passed to `write`.
    ///
    /// Internally, this function ensures that concurrent access to the package
    /// directory is safe. From two concurrent stores, it is not specified which
    /// one wins, but they will not infer or result in partial / corrupted
    /// package contents.
    #[cfg(feature = "universe-packages")]
    pub fn store(
        &self,
        spec: &PackageSpec,
        write: impl FnOnce(&Path) -> PackageResult<()>,
    ) -> PackageResult<()> {
        let error = |message: &str, err: std::io::Error| -> PackageError {
            PackageError::Other(Some(eco_format!("{message}: {err}")))
        };

        // The directory in which the package's version lives.
        let base_dir = self.path().join(format!("{}/{}", spec.namespace, spec.name));

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

        // Non-atomically write the package contents into the temporary
        // directory.
        write(tempdir.as_ref())?;

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
        match std::fs::rename(&tempdir, &package_dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
            Err(err) => Err(error("failed to move downloaded package directory", err)),
        }
    }
}

/// A temporary directory that is a automatically cleaned up.
#[cfg(feature = "universe-packages")]
struct Tempdir(PathBuf);

#[cfg(feature = "universe-packages")]
impl Tempdir {
    /// Creates a directory at the path and auto-cleans it.
    fn create(path: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}

#[cfg(feature = "universe-packages")]
impl Drop for Tempdir {
    fn drop(&mut self) {
        _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(feature = "universe-packages")]
impl AsRef<Path> for Tempdir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Serves packages from the Typst Universe registry.
///
/// There is no standardized registry protocol. This is merely designed to work
/// with the official Typst Universe package registry.
#[cfg(feature = "universe-packages")]
pub struct UniversePackages {
    /// The url of the registry.
    url: String,
    /// A downloader with which we can download from the registry.
    downloader: Box<dyn Downloader>,
    /// The package index.
    index: OnceCell<Box<[serde_json::Value]>>,
}

#[cfg(feature = "universe-packages")]
impl UniversePackages {
    /// The namespace from which Typst Universe serves packages.
    pub const NAMESPACE: &str = "preview";

    /// Creates a new handle for interacting with the primary official registry
    /// at `https://packages.typst.org`.
    pub fn new(downloader: impl Downloader) -> Self {
        Self::with_url(downloader, "https://packages.typst.org")
    }

    /// Creates a new handle which serves packages from an alternative mirror.
    pub fn with_url(downloader: impl Downloader, url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            downloader: Box::new(downloader),
            index: OnceCell::new(),
        }
    }

    /// Returns the registry's URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Attempts to download a package from the registry.
    ///
    /// Will invoke the downloader with the `spec` as the key.
    pub fn package(
        &self,
        spec: &PackageSpec,
    ) -> PackageResult<tar::Archive<impl Read + use<>>> {
        if spec.namespace != Self::NAMESPACE {
            return Err(PackageError::NotFound(spec.clone()));
        }

        let url = format!(
            "{}/{}/{}-{}.tar.gz",
            self.url,
            Self::NAMESPACE,
            spec.name,
            spec.version,
        );

        match self.downloader.download(spec, &url) {
            Ok(data) => {
                let decompressed = flate2::read::GzDecoder::new(Cursor::new(data));
                Ok(tar::Archive::new(decompressed))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Err(match self.latest_version(&spec.versionless()) {
                    Ok(version) => PackageError::VersionNotFound(spec.clone(), version),
                    Err(_) => PackageError::NotFound(spec.clone()),
                })
            }
            Err(err) => Err(PackageError::NetworkFailed(Some(eco_format!("{err}")))),
        }
    }

    /// Attempts to determine the latest version of a package.
    ///
    /// Will invoke the downloader with the key `"package index"`.
    pub fn latest_version(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> StrResult<PackageVersion> {
        /// Minimal information required about a package to determine its latest
        /// version.
        #[derive(Deserialize)]
        struct MinimalPackageInfo {
            name: String,
            version: PackageVersion,
        }

        if spec.namespace != Self::NAMESPACE {
            bail!(
                "failed to determine latest version \
                 (an index is only available for the `{}` namespace)",
                Self::NAMESPACE
            )
        }

        self.index()?
            .iter()
            .filter_map(|value| MinimalPackageInfo::deserialize(value).ok())
            .filter(|package| package.name == spec.name)
            .map(|package| package.version)
            .max()
            .ok_or_else(|| eco_format!("failed to find package {spec}"))
    }

    /// Downloads the package index for the default namespace from the registry
    /// or serves it from its in-memory cache.
    ///
    /// For compatibility, the individual entries are left unserialized. This
    /// way, packages that cannot be deserialized with this compiler version can
    /// be skipped instead of failing completely.
    ///
    /// The index format of the official package registry is not specified or
    /// stabilized and may be changed at any time.
    fn index(&self) -> StrResult<&[serde_json::Value]> {
        self.index
            .get_or_try_init(|| {
                let url = format!("{}/{}/index.json", self.url, Self::NAMESPACE);
                match self.downloader.download(&"package index", &url) {
                    Ok(data) => serde_json::from_slice(&data).map_err(|err| {
                        eco_format!("failed to parse package index: {err}")
                    }),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                        bail!("failed to fetch package index (not found)")
                    }
                    Err(err) => bail!("failed to fetch package index ({err})"),
                }
            })
            .map(AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "universe-packages")]
    fn lazy_deserialize_index() {
        use super::*;
        use std::any::Any;

        struct DummyDownloader;

        impl Downloader for DummyDownloader {
            fn stream(
                &self,
                _: &dyn Any,
                _: &str,
            ) -> std::io::Result<(Option<usize>, Box<dyn Read>)> {
                Err(std::io::ErrorKind::NotFound.into())
            }
        }

        let mut packages = UniversePackages::new(DummyDownloader);
        packages.index = OnceCell::from(Box::new([
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
        ]) as Box<[_]>);

        let ieee_version = packages.latest_version(&VersionlessPackageSpec {
            namespace: "preview".into(),
            name: "charged-ieee".into(),
        });
        assert_eq!(ieee_version, Ok(PackageVersion { major: 0, minor: 1, patch: 0 }));

        let ams_version = packages.latest_version(&VersionlessPackageSpec {
            namespace: "preview".into(),
            name: "unequivocal-ams".into(),
        });
        assert_eq!(
            ams_version,
            Err("failed to find package @preview/unequivocal-ams".into())
        )
    }
}
