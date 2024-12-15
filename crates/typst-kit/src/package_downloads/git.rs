use crate::package_downloads::{DownloadState, PackageDownloader, Progress};
use auth_git2::GitAuthenticator;
use ecow::{eco_format, EcoString};
use git2::build::RepoBuilder;
use git2::{FetchOptions, RemoteCallbacks};
use std::collections::VecDeque;
use std::fmt::Debug;
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

        eprintln!("{} {} {}", repo, tag, dest.display());

        let state = DownloadState {
            content_len: None,
            total_downloaded: 0,
            bytes_per_second: VecDeque::from(vec![0; 5]),
            start_time: Instant::now(),
        };

        let auth = GitAuthenticator::default();
        let git_config = git2::Config::open_default()
            .map_err(|err| EcoString::from(format!("{err}")))?;

        let mut fetch_options = FetchOptions::new();
        let mut remote_callbacks = RemoteCallbacks::new();

        remote_callbacks.credentials(auth.credentials(&git_config));
        fetch_options.remote_callbacks(remote_callbacks);

        let repo = RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(repo, dest)
            .map_err(|err| EcoString::from(format!("{err}")))?;

        let (object, reference) = repo
            .revparse_ext(tag)
            .map_err(|err| EcoString::from(format!("{err}")))?;
        repo.checkout_tree(&object, None)
            .map_err(|err| EcoString::from(format!("{err}")))?;

        match reference {
            // gref is an actual reference like branches or tags
            Some(gref) => repo.set_head(gref.name().unwrap()),
            // this is a commit, not a reference
            None => repo.set_head_detached(object.id()),
        }
        .map_err(|err| EcoString::from(format!("{err}")))?;

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
        let tag = format!("v{}", spec.version);
        self.download_with_progress(repo.as_str(), tag.as_str(), package_dir, progress)
            .map_err(|x| PackageError::Other(Some(x)))
    }
}
