use typst::diag::PackageResult;
use typst_kit::package::PackageStorage;

use crate::args::{PackageStorageArgs, PurgeCommand};
use crate::download;

/// Returns a new package storage for the given args.
pub fn storage(args: &PackageStorageArgs) -> PackageStorage {
    PackageStorage::new(
        args.package_cache_path.clone(),
        args.package_path.clone(),
        download::downloader(),
    )
}

/// Purges the package cache.
pub fn purge(command: &PurgeCommand) -> PackageResult<()> {
    let storage = storage(&command.package_storage_args);

    storage.purge_cache()?;
    println!("Purged package cache");

    if command.local {
        storage.purge_local()?;
        println!("Purged local packages");
    }

    Ok(())
}
