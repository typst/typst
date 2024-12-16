use typst_kit::package::PackageStorage;

use crate::args::PackageArgs;
use crate::download;

/// Returns a new package storage for the given args.
pub fn storage(args: &PackageArgs) -> PackageStorage {
    PackageStorage::new(
        args.package_cache_path.clone(),
        args.package_path.clone(),
        download::downloader(),
    )
}
