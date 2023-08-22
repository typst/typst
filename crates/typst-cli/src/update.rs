use std::{
    env, fs,
    io::{self, BufReader, Cursor, ErrorKind, Read, Seek},
    path::Path,
};

use semver::Version;
use serde::Deserialize;
use typst::{diag::bail, diag::StrResult, eval::eco_format};
use xz2::bufread::XzDecoder;
use zip::ZipArchive;

use crate::args::UpdateCommand;

const TYPST_GITHUB_ORG: &str = "typst";
const TYPST_REPO: &str = "typst";

/// Self update the typst CLI binary.
///
/// Fetches a target release or the latest release (if no version was specified)
/// from GitHub, unpacks it and self replaces the current binary with the
/// pre-compiled asset from the downloaded release.
pub fn update(command: UpdateCommand) -> StrResult<()> {
    match self_update_permitted()? {
        SelfUpdatePermission::Deny => {
            eprintln!("Self-update is disabled for this build of the typst cli");
            eprintln!("You should probably use your system package manager");
            bail!("update failed");
        }
        SelfUpdatePermission::Permit => {}
    }

    if let Some(ref version) = command.version {
        let current_tag = env!("CARGO_PKG_VERSION").parse().unwrap();
        if !command.force && version < &current_tag {
            eprintln!("Certain downgraded Typst versions will not have the update command available");
            eprintln!("Forcing a downgrade might break your installation");
            eprintln!(
                "You can force a downgrade by running `typst update <VERSION> --force`"
            );
            bail!("update failed");
        }
    }

    let current_exe = env::current_exe()
        .map_err(|err| eco_format!("failed to grab current exe path: {}", err))?;

    #[cfg(target_os = "linux")]
    let root_backup_dir = dirs::state_dir()
        .or_else(|| dirs::data_dir())
        .expect("unable to locate local data or state directories");
    #[cfg(not(target_os = "linux"))]
    let root_backup_dir =
        dirs::data_dir().expect("unable to locate local data directory");
    let backup_dir = root_backup_dir.join("typst");
    let backup = backup_dir.join("typst_backup.part");

    fs::create_dir_all(&backup_dir)
        .map_err(|err| eco_format!("failed to create backup directory: {err}"))?;

    if command.revert {
        if !backup.exists() {
            bail!("unable to revert, no backup found (searched in {backup_dir:?})");
        }

        return self_replace::self_replace(&backup)
            .and_then(|_| fs::remove_file(&backup))
            .map_err(|err| eco_format!("unable to revert to backup: {err}"));
    }

    let buffer = command
        .version
        .map_or_else(Release::from_latest, Release::from_tag)
        .and_then(|release| {
            if !update_needed(&release) && !command.force {
                bail!("Already on the latest version");
            }

            Ok(release)
        })
        .and_then(|release| release.download_asset(asset_needed()?))
        .and_then(|binary| binary.unpack())?;

    fs::copy(&current_exe, &backup)
        .map_err(|err| eco_format!("backing up failed: {}", err))?;

    let temp_exe = current_exe
        .parent()
        .unwrap_or(Path::new("."))
        .join("typst_update.part");
    let mut binary_part = fs::File::create(&temp_exe)
        .map_err(|err| eco_format!("failed to create typst_update.part: {}", err))?;
    io::copy(&mut buffer.as_slice(), &mut binary_part).map_err(|err| {
        fs::remove_file(&temp_exe).ok();
        eco_format!("failed to write typst_update.part: {}", err)
    })?;

    self_replace::self_replace(&temp_exe)
        .map_err(|err| {
            fs::remove_file(&temp_exe).ok();
            eco_format!("self replace failed: {}", err)
        })
        .and_then(|_| {
            fs::remove_file(&temp_exe)
                .map_err(|err| eco_format!("failed to delete typst_update.part: {}", err))
        })
}

/// Reflects the posibility of self updating.
#[derive(Clone, Copy, Debug)]
enum SelfUpdatePermission {
    Deny,
    Permit,
}

/// Assets belonging to a GitHub release.
///
/// Primarily used to download pre-compiled Typst CLI binaries.
#[derive(Debug, Deserialize)]
struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// A GitHub release.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

impl Release {
    /// Download the lastest release from the Typst repository.
    pub fn from_latest() -> StrResult<Self> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            TYPST_GITHUB_ORG, TYPST_REPO
        );

        Release::download(&url)
    }

    /// Download the target release from the Typst repository.
    pub fn from_tag(tag: Version) -> StrResult<Release> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/tags/v{}",
            TYPST_GITHUB_ORG, TYPST_REPO, tag
        );

        Release::download(&url)
    }

    fn download(url: &str) -> StrResult<Self> {
        match ureq::get(url).call() {
            Ok(response) => {
                Ok(response.into_json().expect("unable to get json from response"))
            }
            Err(ureq::Error::Status(404, _)) => {
                bail!("release not found (searched at {url})")
            }
            Err(_) => bail!("failed to download release (network failed)"),
        }
    }

    /// Sorts through the assets from a given [`Release`] and picks the right one
    /// for the target platform, returning its packed binary.
    pub fn download_asset(&self, asset_name: &str) -> StrResult<PackedBinary> {
        let asset = self
            .assets
            .iter()
            .find(|a| a.name.starts_with(asset_name))
            .ok_or("could not find release for your target platform")?;

        let response = match ureq::get(&asset.browser_download_url).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(404, _)) => {
                bail!("asset not found (searched for {})", asset.name);
            }
            Err(_) => bail!("failed to load asset (network failed)"),
        };

        let mut buffer = response
            .header("Content-Length")
            .and_then(|header| header.parse().ok())
            .map_or_else(Vec::new, Vec::with_capacity);
        response
            .into_reader()
            .read_to_end(&mut buffer)
            .map_err(|err| eco_format!("failed to read response buffer: {err}"))?;

        Ok(PackedBinary(buffer))
    }
}

/// Extension trait that targets the Typst binary and unpacks it.
trait Unpack {
    fn unpack_typst_binary(&mut self) -> StrResult<Vec<u8>>;
}

impl<R: Read + Seek> Unpack for ZipArchive<R> {
    fn unpack_typst_binary(&mut self) -> StrResult<Vec<u8>> {
        let mut binary = self
            .by_name(&format!("{}/typst.exe", asset_needed()?))
            .map_err(|_| "asset archive did not contain typst binary")?;

        let mut buffer = Vec::with_capacity(binary.size() as usize);
        binary
            .read_to_end(&mut buffer)
            .map_err(|err| eco_format!("failed to read binary data: {}", err))?;

        Ok(buffer)
    }
}

impl<R: Read> Unpack for tar::Archive<R> {
    fn unpack_typst_binary(&mut self) -> StrResult<Vec<u8>> {
        let mut binary = self
            .entries()
            .map_err(|err| eco_format!("xz archive corrupted: {}", err))?
            .filter_map(Result::ok)
            .find(|e| e.path().unwrap_or_default().ends_with("typst"))
            .ok_or("asset archive did not contain typst binary")?;

        let mut buffer = Vec::with_capacity(binary.size() as usize);
        binary
            .read_to_end(&mut buffer)
            .map_err(|err| eco_format!("failed to read binary data: {}", err))?;

        Ok(buffer)
    }
}

/// The raw binary data from a packed [`Asset`].
struct PackedBinary(Vec<u8>);

impl PackedBinary {
    /// Unpacks the asset archive in-memory and writes the uncompressed contents
    /// into a buffer.
    ///
    /// In-memory refers to that the physical archive is never written to disk,
    /// only the Typst CLI binary will be plucked from the archive and written
    /// to disk at a later stage, the rest of the archive is discarded.
    pub fn unpack(self) -> StrResult<Vec<u8>> {
        let mut raw = BufReader::new(Cursor::new(self.0));

        tar::Archive::new(XzDecoder::new(raw.by_ref()))
            .unpack_typst_binary()
            .ok()
            .or_else(|| {
                ZipArchive::new(raw)
                    .ok()
                    .and_then(|mut archive| archive.unpack_typst_binary().ok())
            })
            .ok_or("asset archive unknown or corrupted".into())
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

/// Compares latest release version to current version to see if an update
/// is even needed
fn update_needed(release: &Release) -> bool {
    let current_tag: Version = env!("CARGO_PKG_VERSION").parse().unwrap();
    let new_tag: Version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .parse()
        .expect("release tag not in semver format");

    new_tag > current_tag
}

/// A shallow check to see if we have the proper permissions to create files
/// in the current working directory.
fn self_update_permitted() -> StrResult<SelfUpdatePermission> {
    if cfg!(windows) {
        Ok(SelfUpdatePermission::Permit)
    } else {
        let current_exe = env::current_exe()
            .map_err(|err| eco_format!("failed to grab current exe path: {}", err))?;
        let current_exe_dir =
            current_exe.parent().expect("typst cli isn't in a directory");
        if let Err(e) =
            tempfile::Builder::new().prefix("updtest").tempdir_in(current_exe_dir)
        {
            match e.kind() {
                ErrorKind::PermissionDenied => {
                    return Ok(SelfUpdatePermission::Deny);
                }
                _ => return Err(e.to_string().into()),
            }
        }

        Ok(SelfUpdatePermission::Permit)
    }
}
