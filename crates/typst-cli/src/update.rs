use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufReader, Cursor, ErrorKind, Read},
    path::Path,
};

use semver::Version;
use serde::Deserialize;
use typst::{diag::bail, diag::StrResult, eval::eco_format};

use crate::args::UpdateCommand;

// these might not be very usefull but does make it easier to maintain
// if the organization/repo moves or changes (only used in release fetching)
const TYPST_GITHUB_ORG: &str = "typst";
const TYPST_REPO: &str = "typst";

#[cfg(feature = "no-self-update")]
pub(crate) const NEVER_SELF_UPDATE: bool = true;
#[cfg(not(feature = "no-self-update"))]
pub(crate) const NEVER_SELF_UPDATE: bool = false;

/// Figure out if there are sufficient permissions to carry out an update -- 
/// hard failing if the cli is installed through a package manager.
#[derive(Clone, Copy, Debug)]
enum SelfUpdatePermission {
    HardFail,
    Permit,
}

/// A GitHub release.
#[derive(Debug, Deserialize)]
struct Release {
    name: String,
    tag_name: String,
    assets: Vec<Asset>,
}

/// Assets that were uploaded to a GitHub release.
///
/// Primarly used to download pre-compiled typst CLI binaries.
#[derive(Debug, Deserialize)]
struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// GitHub asset archive with the typst CLI executable as unpacked data.
#[derive(Debug)]
struct Archive {
    pub extension: Extension,
    pub buffer: BufReader<Cursor<Vec<u8>>>,
}

/// The extension for the downloaded archive.
///
/// Possible variations are `zip` and `xz`, other variations are mapped to
/// `Unsupported` and will throw an error.
#[derive(Debug)]
enum Extension {
    Zip,
    Xz,
    Unsupported,
}

impl From<&str> for Extension {
    fn from(value: &str) -> Self {
        match value {
            "zip" => Extension::Zip,
            "xz" => Extension::Xz,
            _ => Extension::Unsupported,
        }
    }
}

/// Self update the typst CLI binary.
///
/// Fetches a target release or the latest release (if no version was specified)
/// from GitHub, unpacks it and self replaces the current binary with the
/// pre-compiled asset from the downloaded release.
pub fn update(command: UpdateCommand) -> StrResult<()> {
    let update_permitted = if NEVER_SELF_UPDATE {
        SelfUpdatePermission::HardFail
    } else {
        self_update_permitted()?
    };

    match update_permitted {
        SelfUpdatePermission::HardFail => {
            println!("self-update is disabled for this build of the typst cli");
            println!("you should probably use your system package manager to update typst");
            // not really an ok scenario but not really an error either?
            return Ok(());
        }
        SelfUpdatePermission::Permit => {}
    }

    // first we check if a downgrade is happening
    if let Some(ref version) = command.version {
        let current_tag = env!("CARGO_PKG_VERSION").parse().unwrap();
        if !command.force && version < &current_tag {
            println!("Certain downgraded typst versions will not have the update command available");
            println!("Forcing a downgrade might break your install");
            println!("You can downgrade by running `typst update <VERSION> --force`");

            return Ok(());
        }
    }

    let executable = env::current_exe()
        .map_err(|err| eco_format!("failed to grab current exe path: {}", err))?;
    let backup = executable
        .parent()
        .unwrap_or(Path::new("./"))
        .join("typst_backup.part");

    // revert to the backed up binary if there is one form a previous update
    if command.revert {
        if !backup.exists() {
            bail!("there is no backup to revert to");
        }

        self_replace::self_replace(&backup)
            .map_err(|err| eco_format!("failed to revert: {}", err))?;
        fs::remove_file(&backup)
            .map_err(|err| eco_format!("failed to remove backup: {}", err))?;

        return Ok(());
    }

    // copy the current executable binary data to typst_backup.part
    // to maintain a backup
    fs::copy(&executable, &backup)
        .map_err(|err| eco_format!("backup creation failed: {}", err))?;

    // get either a target release or latest release through the GitHub API
    let release = match command.version {
        Some(version) => target_release(version)?,
        None => latest_release()?,
    };

    // checks the chosen release tag against typsts package version, if latest
    // no update is required and downgrading is already handled at this point
    if !update_needed(&release) {
        println!("Already on the latest version");
        return Ok(());
    }

    // find the right asset for the target platform and download it
    let archive = download_asset_archive(&release)?;

    // get the typst binary out of their respective archives in-memory and write
    // the binary data to a buffer once it has been unpacked
    let buffer = unpack_archive(archive)?;

    // take the unpacked binary data and copy it into typst_update.part
    let temp_exe = executable
        .parent()
        .unwrap_or(Path::new("./"))
        .join("typst_update.part");
    let mut binary_part = fs::File::create(&temp_exe)
        .map_err(|err| eco_format!("failed to create typst_update.part: {}", err))?;
    std::io::copy(&mut buffer.as_slice(), &mut binary_part).map_err(|err| {
        fs::remove_file(&temp_exe).ok();
        eco_format!("failed to write typst_update.part: {}", err)
    })?;

    // self replace the binary with the data from typst_update.part
    self_replace::self_replace(&temp_exe).map_err(|err| {
        fs::remove_file(&temp_exe).ok();
        eco_format!("failed to self replace binary: {}", err)
    })?;
    // remove the temp typst_update.part artifact
    fs::remove_file(&temp_exe)
        .map_err(|err| eco_format!("failed to delete typst_update.part: {}", err))?;

    // done, typst updated itself
    println!("typst updated successfully: {}", release.name);

    Ok(())
}

/// Specifies the url for the target release.
fn target_release(version: Version) -> StrResult<Release> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/tags/v{}",
        TYPST_GITHUB_ORG, TYPST_REPO, version
    );

    download_release(&url)
}

/// Specifies the url for the latest release.
fn latest_release() -> StrResult<Release> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        TYPST_GITHUB_ORG, TYPST_REPO
    );

    download_release(&url)
}

/// Downloads and parses a GitHub release from the Typst repository.
fn download_release(url: &str) -> StrResult<Release> {
    match ureq::get(url).call() {
        Ok(response) => {
            Ok(response.into_json().map_err(|_| "unable to get json from response")?)
        }
        Err(ureq::Error::Status(404, _)) => {
            // TODO: maybe make enum error type in typst::diag for this?
            bail!("release not found")
        }
        // TODO: same as above
        Err(_) => bail!("network failed"),
    }
}

/// Sorts through the assets from a given `Release` and picks the right one
/// for this target platform.
///
/// Returns a compressed archive that contains the Typst pre-compiled binary.
fn download_asset_archive(release: &Release) -> StrResult<Archive> {
    let asset_needed = asset_needed()?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.starts_with(asset_needed))
        .ok_or("could not find release for your target platform.")?;

    let response = match ureq::get(&asset.browser_download_url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(404, _)) => {
            // TODO: maybe make enum error type in typst::diag for this?
            bail!("asset not found");
        }
        // TODO: same as above
        Err(_) => bail!("network failed"),
    };

    let len = response.header("Content-Length").unwrap().parse().unwrap();
    let mut buffer = Vec::with_capacity(len);
    response.into_reader().read_to_end(&mut buffer).unwrap();

    let extension = Path::new(&asset.name)
        .extension()
        .and_then(OsStr::to_str)
        .ok_or("failed to get extension from archive")?
        .into();

    Ok(Archive {
        extension,
        buffer: BufReader::new(Cursor::new(buffer)),
    })
}

/// Unpacks the asset archive in-memory and writes the uncompressed contents
/// into a buffer.
///
/// In-memory refers to that the physical archive is never written to disk, only
/// the Typst CLI binary will be plucked from the archive and written to disk at
/// a later stage, the rest of the archive is discarded.
fn unpack_archive(archive: Archive) -> StrResult<Vec<u8>> {
    match archive.extension {
        Extension::Zip => {
            let mut zip = zip::ZipArchive::new(archive.buffer)
                .map_err(|err| eco_format!("Error opening zip archive: {}", err))?;
            let mut binary = zip
                .by_name(&format!("{}/typst.exe", asset_needed()?))
                .map_err(|_| "asset archive did not contain typst binary")?;

            let mut buffer = Vec::with_capacity(binary.size() as usize);
            binary
                .read_to_end(&mut buffer)
                .map_err(|err| eco_format!("failed to read binary data: {}", err))?;

            Ok(buffer)
        }
        Extension::Xz => {
            let decompressed = xz2::read::XzDecoder::new(archive.buffer);
            let mut archive = tar::Archive::new(decompressed);

            // FIXME: this is a bit of a mess
            // still trying to figure out how to get the binary out of the archive
            // a bit more gracefully :)
            let mut binary = archive
                .entries()
                .map_err(|err| eco_format!("xz archive is empty: {}", err))?
                .find(|e| e.as_ref().unwrap().path().unwrap().ends_with("typst"))
                .unwrap()
                .unwrap();

            let mut buffer = Vec::with_capacity(binary.size() as usize);
            binary
                .read_to_end(&mut buffer)
                .map_err(|err| eco_format!("failed to read binary data: {}", err))?;

            Ok(buffer)
        }
        _ => bail!("asset archive format unsupported"),
    }
}

/// Determines what asset to download according to the target platform the CLI
/// is running on.
fn asset_needed() -> StrResult<&'static str> {
    Ok(match env!("TARGET") {
        "x86_64-unknown-linux-gnu" => "typst-x86_64-unknown-linux-musl",
        "x86_64-unknown-linux-musl" => "typst-x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl" => "typst-aarch64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu" => "typst-aarch64-unknown-linux-musl",
        "armv7-unknown-linux-musleabi" => "typst-armv7-unknown-linux-musleabi",
        "x86_64-apple-darwin" => "typst-x86_64-apple-darwin",
        "aarch64-apple-darwin" => "typst-aarch64-apple-darwin",
        "x86_64-pc-windows-msvc" => "typst-x86_64-pc-windows-msvc",
        target => bail!("unsupported target: {target}"),
    })
}

/// Early return check to see if the CLI even needs updating.
fn update_needed(release: &Release) -> bool {
    let current_tag: Version = env!("CARGO_PKG_VERSION").parse().unwrap();
    // TODO: https://github.com/typst/typst/blob/2f81089995c87efdbce6c94bb29647cd1f213cfd/crates/typst-cli/src/world.rs#L69
    let new_tag: Version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .parse()
        .unwrap();

    new_tag > current_tag
}

fn self_update_permitted() -> StrResult<SelfUpdatePermission> {
    if cfg!(windows) {
        Ok(SelfUpdatePermission::Permit)
    } else {
        let current_exe = env::current_exe()
            .map_err(|err| eco_format!("failed to grab current exe path: {}", err))?;
        let current_exe_dir = current_exe.parent().expect("typst cli isn't in a directory‽");
        if let Err(e) =
            tempfile::Builder::new().prefix("updtest").tempdir_in(current_exe_dir)
        {
            match e.kind() {
                ErrorKind::PermissionDenied => {
                    return Ok(SelfUpdatePermission::HardFail);
                }
                _ => return Err(e.to_string().into()),
            }
        }
        
        Ok(SelfUpdatePermission::Permit)
    }
}
