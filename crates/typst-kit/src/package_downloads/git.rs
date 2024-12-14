use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs;
use std::io::{self, ErrorKind, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use auth_git2::GitAuthenticator;
use crate::package_downloads::{
    DownloadState, PackageDownloader, Progress, DEFAULT_NAMESPACE,
};
use ecow::{eco_format, EcoString};
use native_tls::{Certificate, TlsConnector};
use once_cell::sync::OnceCell;
use typst_library::diag::{bail, PackageError, PackageResult};
use typst_syntax::package::{PackageInfo, PackageSpec, VersionlessPackageSpec};
use git2::{AutotagOption, FetchOptions, Progress as GitProgress, RemoteCallbacks};
use git2::build::{CheckoutBuilder, RepoBuilder};
use typst_library::html::tag::form;

#[derive(Debug)]
pub struct GitDownloader;

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

        let state = DownloadState{
            content_len: None,
            total_downloaded: 0,
            bytes_per_second: VecDeque::from(vec![0; 5]),
            start_time: Instant::now(),
        };


        let auth = GitAuthenticator::default();
        let git_config = git2::Config::open_default().map_err(|err| {EcoString::from(format!("{:?}", err))})?;

        let mut fetch_options = FetchOptions::new();
        let mut remote_callbacks = RemoteCallbacks::new();

        remote_callbacks.credentials(auth.credentials(&git_config));
        fetch_options
            .remote_callbacks(remote_callbacks);

        let repo = RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(repo, dest).map_err(|err| {EcoString::from(format!("{:?}", err))})?;

        let (object, reference) = repo
            .revparse_ext(tag).map_err(|err| {EcoString::from(format!("{:?}", err))})?;
        repo.checkout_tree(&object, None).map_err(|err| {EcoString::from(format!("{:?}", err))})?;

        match reference {
            // gref is an actual reference like branches or tags
            Some(gref) => repo.set_head(gref.name().unwrap()),
            // this is a commit, not a reference
            None => repo.set_head_detached(object.id()),
        }.map_err(|err| {EcoString::from(format!("{:?}", err))})?;

        progress.print_finish(&state);
        Ok(())
    }

    fn parse_namespace(ns: &str, name: &str) -> Result<String, EcoString> {

        let mut parts = ns.splitn(2, ":");
        let schema = parts.next().ok_or_else(|| {
            eco_format!("expected schema in {}", ns)
        })?;
        let repo = parts.next().ok_or_else(|| {
            eco_format!("invalid package repo {}", ns)
        })?;

        if !schema.eq("git") {
            Err(eco_format!("invalid schema in {}", ns))?
        }

        Ok(format!("{}/{}.git", repo, name))
    }
}

impl PackageDownloader for GitDownloader {
    fn download_index(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> Result<Vec<PackageInfo>, EcoString> {

        //todo ls-remote
        todo!()
    }

    fn download(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()> {
        let repo = Self::parse_namespace(spec.namespace.as_str(), spec.name.as_str()).map_err(|x| PackageError::Other(Some(x)))?;
        let tag = format!("v{}", spec.version);
        self.download_with_progress(repo.as_str(), tag.as_str(), package_dir, progress).map_err(|x| PackageError::Other(Some(x)))
    }
}
