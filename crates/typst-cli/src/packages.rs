use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};

use crate::args::PackageArgs;

/// Returns a new package storage for the given args.
pub fn system(args: &PackageArgs) -> SystemPackages {
    SystemPackages::from_parts(
        args.package_path
            .clone()
            .map(FsPackages::new)
            .or_else(FsPackages::system_data),
        args.package_cache_path
            .clone()
            .map(FsPackages::new)
            .or_else(FsPackages::system_cache),
        UniversePackages::new(crate::download::downloader()),
    )
}
