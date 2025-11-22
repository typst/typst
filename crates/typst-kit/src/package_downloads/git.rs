use crate::package_downloads::{DownloadState, PackageDownloader, Progress};
use ecow::{EcoString, eco_format};
use gix::remote::fetch::Shallow;
use std::fmt::Debug;
use std::num::NonZero;
use std::path::Path;
use std::time::Instant;
use typst_library::diag::{PackageError, PackageResult};
use typst_syntax::package::{PackageInfo, PackageSpec, VersionlessPackageSpec};

#[derive(Debug)]
pub struct GitDownloader;

impl Default for GitDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl GitDownloader {
    pub fn new() -> Self {
        Self {}
    }

    pub fn download_with_progress(
        &self,
        repo: &str,
        tag: &str,
        dest: &Path,
        progress: &mut dyn Progress,
    ) -> Result<(), EcoString> {
        progress.print_start();
        let state = DownloadState {
            content_len: None,
            total_downloaded: 0,
            bytes_per_second: Default::default(),
            start_time: Instant::now(),
        };

        std::fs::create_dir_all(dest).map_err(|x| eco_format!("{x}"))?;
        let url = gix::url::parse(repo.into()).map_err(|x| eco_format!("{x}"))?;
        let mut prepare_fetch =
            gix::prepare_clone(url, dest).map_err(|x| eco_format!("{x}"))?;
        prepare_fetch = prepare_fetch
            .with_shallow(Shallow::DepthAtRemote(NonZero::new(1).unwrap()))
            .with_ref_name(Some(tag))
            .map_err(|x| eco_format!("{x}"))?;

        let (mut prepare_checkout, _) = prepare_fetch
            .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|x| eco_format!("{x}"))?;
        if prepare_checkout.repo().work_dir().is_none() {
            return Err(eco_format!(
                "Cloned git repository but files are not available."
            ))?;
        }

        prepare_checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|x| eco_format!("{x}"))?;
        progress.print_finish(&state);
        Ok(())
    }

    /// Parses the namespace of the package into the correct registry and namespace.
    /// The namespace format is the following:
    ///
    /// @git:<git host and user>
    ///
    /// The final repository cloned will be formed by the git host and the repository name
    /// with the adequate extension, checking out to the tag specified by the version in the format
    /// v<major>.<minor>.<patch>
    ///
    /// For example, the package
    /// @git:git@github.com:typst/package:0.0
    /// will result in the cloning of the repository git@github.com:typst/package.git
    /// and the checkout and detached head state at tag v0.1.0
    ///
    /// NOTE: no index download is possible.
    fn parse_namespace(ns: &str, name: &str) -> Result<String, EcoString> {
        let mut parts = ns.splitn(2, ":");
        let schema =
            parts.next().ok_or_else(|| eco_format!("expected schema in {}", ns))?;
        let repo = parts
            .next()
            .ok_or_else(|| eco_format!("invalid package repo {}", ns))?;

        if !schema.eq("git") {
            Err(eco_format!("invalid schema in {}", ns))?
        }

        Ok(format!("{repo}/{name}.git"))
    }
}

impl PackageDownloader for GitDownloader {
    fn download_index(
        &self,
        _spec: &VersionlessPackageSpec,
    ) -> Result<Vec<PackageInfo>, EcoString> {
        Err(eco_format!("Downloading index is not supported for git repositories"))
    }

    fn download(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()> {
        let repo = Self::parse_namespace(spec.namespace.as_str(), spec.name.as_str())
            .map_err(|x| PackageError::Other(Some(x)))?;
        let tag = format!("refs/tags/v{}", spec.version);
        self.download_with_progress(repo.as_str(), tag.as_str(), package_dir, progress)
            .map_err(|x| PackageError::Other(Some(x)))
    }
}
