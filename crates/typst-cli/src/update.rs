use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use ecow::eco_format;
use semver::Version;
use serde::Deserialize;
use tempfile::NamedTempFile;
use typst::diag::{StrResult, bail};
use typst_kit::download::Downloader;
use xz2::bufread::XzDecoder;
use zip::ZipArchive;

use crate::args::UpdateCommand;
use crate::download::{self, PrintDownload};

const TYPST_GITHUB_ORG: &str = "typst";
const TYPST_REPO: &str = "typst";

/// Determine the asset to download based on the target platform.
///
/// See `.github/workflows/release.yml` for the list of prebuilt assets.
macro_rules! determine_asset {
    () => {
        // For some platforms, only some targets are prebuilt in the release.
        determine_asset!(__impl: {
            "x86_64-unknown-linux-gnu" => "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-gnu" => "aarch64-unknown-linux-musl",
            "armv7-unknown-linux-gnueabi" => "armv7-unknown-linux-musleabi",
            "riscv64gc-unknown-linux-musl" => "riscv64gc-unknown-linux-gnu",
        })
    };

    (__impl: { $($origin:literal => $target:literal),* $(,)? }) => {
        match env!("TARGET") {
            $($origin => concat!("typst-", $target),)*
            _ => concat!("typst-", env!("TARGET")),
        }
    };
}

/// Self update the Typst CLI binary.
///
/// Fetches a target release or the latest release (if no version was specified)
/// from GitHub, unpacks it and self replaces the current binary with the
/// pre-compiled asset from the downloaded release.
pub fn update(command: &UpdateCommand) -> StrResult<()> {
    if let Some(ref version) = command.version {
        // NOTE: Although [`typst_syntax::TypstVersion`] uses the [`semver`] crate under the hood
        // right now, we consider this an implementation detail that is not currently exposed.
        let current_tag = typst::version().raw().parse().unwrap();

        if version < &Version::new(0, 8, 0) {
            eprintln!(
                "note: versions older than 0.8.0 will not have \
                 the update command available."
            );
        }

        if !command.force && version < &current_tag {
            bail!(
                "downgrading requires the --force flag: \
                `typst update <VERSION> --force`",
            );
        }
    }

    // Full path to the backup file.
    let backup_path = command.backup_path.clone().map(Ok).unwrap_or_else(backup_path)?;

    if let Some(backup_dir) = backup_path.parent() {
        fs::create_dir_all(backup_dir)
            .map_err(|err| eco_format!("failed to create backup directory ({err})"))?;
    }

    if command.revert {
        if !backup_path.exists() {
            bail!(
                "unable to revert, no backup found (searched at {})",
                backup_path.display(),
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

    let downloader = download::downloader();

    let release = Release::from_tag(command.version.as_ref(), &downloader)?;
    if !update_needed(&release)? && !command.force {
        eprintln!("Already up-to-date.");
        return Ok(());
    }

    let binary_data = release.download_binary(determine_asset!(), &downloader)?;
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
    pub fn from_tag(
        tag: Option<&Version>,
        downloader: &Downloader,
    ) -> StrResult<Release> {
        let url = match tag {
            Some(tag) => format!(
                "https://api.github.com/repos/{TYPST_GITHUB_ORG}/{TYPST_REPO}/releases/tags/v{tag}"
            ),
            None => format!(
                "https://api.github.com/repos/{TYPST_GITHUB_ORG}/{TYPST_REPO}/releases/latest",
            ),
        };

        match downloader.download(&url) {
            Ok(response) => response.into_json().map_err(|err| {
                eco_format!("failed to parse release information ({err})")
            }),
            Err(ureq::Error::Status(404, _)) => {
                bail!("release not found (searched at {url})")
            }
            Err(err) => bail!("failed to download release ({err})"),
        }
    }

    /// Download the binary from a given [`Release`] and select the
    /// corresponding asset for this target platform, returning the raw binary
    /// data.
    pub fn download_binary(
        &self,
        asset_name: &str,
        downloader: &Downloader,
    ) -> StrResult<Vec<u8>> {
        let asset = self.assets.iter().find(|a| a.name.starts_with(asset_name)).ok_or(
            eco_format!(
                "could not find prebuilt binary `{}` for your platform",
                asset_name
            ),
        )?;

        let data = match downloader.download_with_progress(
            &asset.browser_download_url,
            &mut PrintDownload("release"),
        ) {
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

    let mut file =
        archive.by_name(&format!("{asset_name}/typst.exe")).map_err(|err| {
            eco_format!("failed to extract Typst binary from ZIP archive ({err})")
        })?;

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

/// Compare the release version to the CLI version to see if an update is needed.
fn update_needed(release: &Release) -> StrResult<bool> {
    // NOTE: Although [`typst_syntax::TypstVersion`] uses the [`semver`] crate under the hood right
    // now, we consider this an implementation detail that is not currently exposed.
    let current_tag: Version = typst::version().raw().parse().unwrap();
    let new_tag: Version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .parse()
        .map_err(|_| "release tag not in semver format")?;

    Ok(new_tag > current_tag)
}

/// Path to a potential backup file in the system.
///
/// The backup will be placed as `typst_backup.part` in one of the following
/// directories, depending on the platform:
///  - `$XDG_STATE_HOME` or `~/.local/state` on Linux
///    - `$XDG_DATA_HOME` or `~/.local/share` if the above path isn't available
///  - `~/Library/Application Support` on macOS
///  - `%APPDATA%` on Windows
///
/// If a custom backup path is provided via the environment variable
/// `TYPST_UPDATE_BACKUP_PATH`, it will be used instead of the default
/// directories determined by the platform. In that case, this function
/// shouldn't be called.
fn backup_path() -> StrResult<PathBuf> {
    #[cfg(target_os = "linux")]
    let root_backup_dir = dirs::state_dir()
        .or_else(dirs::data_dir)
        .ok_or("unable to locate local data or state directory")?;

    #[cfg(not(target_os = "linux"))]
    let root_backup_dir =
        dirs::data_dir().ok_or("unable to locate local data directory")?;

    Ok(root_backup_dir.join("typst").join("typst_backup.part"))
}
