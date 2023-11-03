use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use semver::Version;
use serde::Deserialize;
use tempfile::NamedTempFile;
use typst::{diag::bail, diag::StrResult, eval::eco_format};
use xz2::bufread::XzDecoder;
use zip::ZipArchive;

use crate::args::UpdateCommand;
use crate::download::download_with_progress;

const TYPST_GITHUB_ORG: &str = "typst";
const TYPST_REPO: &str = "typst";

/// Self update the Typst CLI binary.
///
/// Fetches a target release or the latest release (if no version was specified)
/// from GitHub, unpacks it and self replaces the current binary with the
/// pre-compiled asset from the downloaded release.
pub fn update(command: &UpdateCommand) -> StrResult<()> {
    if let Some(ref version) = command.version {
        let current_tag = env!("CARGO_PKG_VERSION").parse().unwrap();

        if version < &Version::new(0, 8, 0) {
            eprintln!(
                "Note: Versions older than 0.8.0 will not have \
                 the update command available."
            );
        }

        if !command.force && version < &current_tag {
            bail!(
                "downgrading requires the --force flag: \
                `typst update <VERSION> --force`"
            );
        }
    }

    let backup_path = backup_path()?;
    if command.revert {
        if !backup_path.exists() {
            bail!(
                "unable to revert, no backup found (searched at {})",
                backup_path.display()
            );
        }

        return self_replace::self_replace(&backup_path)
            .and_then(|_| fs::remove_file(&backup_path))
            .map_err(|err| eco_format!("failed to revert to backup ({err})"));
    }

    let current_exe = env::current_exe().map_err(|err| {
        eco_format!("failed to locate path of the running executable ({err})")
    })?;

    fs::copy(current_exe, &backup_path)
        .map_err(|err| eco_format!("failed to create backup ({err})"))?;

    let release = Release::from_tag(command.version.as_ref())?;
    if !update_needed(&release)? && !command.force {
        eprintln!("Already up-to-date.");
        return Ok(());
    }

    let binary_data = release.download_binary(needed_asset()?)?;
    let mut temp_exe = NamedTempFile::new()
        .map_err(|err| eco_format!("failed to create temporary file ({err})"))?;
    temp_exe
        .write_all(&binary_data)
        .map_err(|err| eco_format!("failed to write binary data ({err})"))?;

    self_replace::self_replace(&temp_exe).map_err(|err| {
        fs::remove_file(&temp_exe).ok();
        eco_format!("failed to self-replace running executable ({err})")
    })
}

/// Assets belonging to a GitHub release.
///
/// Primarily used to download pre-compiled Typst CLI binaries.
#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// A GitHub release.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

impl Release {
    /// Download the target release, or latest if version is `None`, from the
    /// Typst repository.
    pub fn from_tag(tag: Option<&Version>) -> StrResult<Release> {
        let url = match tag {
            Some(tag) => format!(
                "https://api.github.com/repos/{}/{}/releases/tags/v{}",
                TYPST_GITHUB_ORG, TYPST_REPO, tag
            ),
            None => format!(
                "https://api.github.com/repos/{}/{}/releases/latest",
                TYPST_GITHUB_ORG, TYPST_REPO
            ),
        };

        match ureq::get(&url).call() {
            Ok(response) => response
                .into_json()
                .map_err(|err| eco_format!("unable to parse JSON response: {err}")),
            Err(ureq::Error::Status(404, _)) => {
                bail!("release not found (searched at {url})")
            }
            Err(err) => bail!("failed to download release ({err})"),
        }
    }

    /// Download the binary from a given [`Release`] and select the
    /// corresponding asset for this target platform, returning the raw binary
    /// data.
    pub fn download_binary(&self, asset_name: &str) -> StrResult<Vec<u8>> {
        let asset = self
            .assets
            .iter()
            .find(|a| a.name.starts_with(asset_name))
            .ok_or("could not find release for your target platform")?;

        eprintln!("Downloading release ...");
        let data = match download_with_progress(&asset.browser_download_url) {
            Ok(data) => data,
            Err(ureq::Error::Status(404, _)) => {
                bail!("asset not found (searched for {})", asset.name);
            }
            Err(err) => bail!("failed to download asset ({err})"),
        };

        if asset_name.contains("windows") {
            extract_binary_from_zip(&data, asset_name)
        } else {
            extract_binary_from_tar_xz(&data)
        }
    }
}

/// Extract the Typst binary from a ZIP archive.
fn extract_binary_from_zip(data: &[u8], asset_name: &str) -> StrResult<Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(data))
        .map_err(|err| eco_format!("failed to extract ZIP archive ({err})"))?;

    let mut file = archive
        .by_name(&format!("{asset_name}/typst.exe"))
        .map_err(|_| "ZIP archive did not contain Typst binary")?;

    let mut buffer = vec![];
    file.read_to_end(&mut buffer).map_err(|err| {
        eco_format!("failed to read binary data from ZIP archive ({err})")
    })?;

    Ok(buffer)
}

/// Extract the Typst binary from a `.tar.xz` archive.
fn extract_binary_from_tar_xz(data: &[u8]) -> StrResult<Vec<u8>> {
    let mut archive = tar::Archive::new(XzDecoder::new(Cursor::new(data)));

    let mut file = archive
        .entries()
        .map_err(|err| eco_format!("failed to extract tar.xz archive ({err})"))?
        .filter_map(Result::ok)
        .find(|e| e.path().unwrap_or_default().ends_with("typst"))
        .ok_or("tar.xz archive did not contain Typst binary")?;

    let mut buffer = vec![];
    file.read_to_end(&mut buffer).map_err(|err| {
        eco_format!("failed to read binary data from tar.xz archive ({err})")
    })?;

    Ok(buffer)
}

/// Determine what asset to download according to the target platform the CLI
/// is running on.
fn needed_asset() -> StrResult<&'static str> {
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

/// Compare the release version to the CLI version to see if an update is needed.
fn update_needed(release: &Release) -> StrResult<bool> {
    let current_tag: Version = env!("CARGO_PKG_VERSION").parse().unwrap();
    let new_tag: Version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .parse()
        .map_err(|_| "release tag not in semver format")?;

    Ok(new_tag > current_tag)
}

/// Path to a potential backup file.
///
/// The backup will be placed in one of the following directories, depending on
/// the platform:
///  - `$XDG_STATE_HOME` or `~/.local/state` on Linux
///    - `$XDG_DATA_HOME` or `~/.local/share` if the above path isn't available
///  - `~/Library/Application Support` on macOS
///  - `%APPDATA%` on Windows
fn backup_path() -> StrResult<PathBuf> {
    #[cfg(target_os = "linux")]
    let root_backup_dir = dirs::state_dir()
        .or_else(|| dirs::data_dir())
        .ok_or("unable to locate local data or state directory")?;

    #[cfg(not(target_os = "linux"))]
    let root_backup_dir =
        dirs::data_dir().ok_or("unable to locate local data directory")?;

    let backup_dir = root_backup_dir.join("typst");

    fs::create_dir_all(&backup_dir)
        .map_err(|err| eco_format!("failed to create backup directory ({err})"))?;

    Ok(backup_dir.join("typst_backup.part"))
}
