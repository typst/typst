//! This module provides the package downloader abstraction needed
//! for remote package handling.
//!
//! # Content
//!
//! ## Traits
//! The [PackageDownloader] trait provides the abstraction needed to implement
//! multiple download method handlers.
//! Each method must allow for a package download to the local filesystem and it should provide a
//! method for downloading the repository index if it exists.
//!
//! The [Progress] trait allows for the implementation of a progress reporting struct.
//!
//! ## Module
//! [http] contains functionality for making simple web requests with status reporting,
//! useful for downloading packages from package registries.
//! It is enabled by the `downloads_http` feature flag.
//! Additionally the `vendor-openssl` can be used on operating systems other than macOS
//! and Windows to vendor OpenSSL when building.
//!
//! [git] contains functionality for handling package downloads through git repositories.

use ecow::{EcoString, eco_format};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::time::Instant;
use typst_library::diag::{PackageError, PackageResult};
use typst_syntax::package::{PackageInfo, PackageSpec, VersionlessPackageSpec};

/// The public namespace in the default Typst registry.
pub const DEFAULT_NAMESPACE: &str = "preview";

/*========BEGIN DOWNLOAD METHODS DECLARATION=========*/
#[cfg(feature = "downloads_http")]
pub mod http;

#[cfg(feature = "downloads_git")]
pub mod git;
/*========END DOWNLOAD METHODS DECLARATION===========*/

/// Trait abstraction for package a downloader.
pub trait PackageDownloader: Debug + Sync + Send {
    /// Download the repository index and returns the
    /// list of PackageInfo elements contained in it.
    fn download_index(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> Result<Vec<PackageInfo>, EcoString>;

    /// Download a package from a remote repository/registry
    /// and writes it in the file system cache directory
    fn download(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()>;
}

/// The current state of an in progress or finished download.
#[derive(Debug)]
pub struct DownloadState {
    /// The expected amount of bytes to download, `None` if the response header
    /// was not set.
    pub content_len: Option<usize>,
    /// The total amount of downloaded bytes until now.
    pub total_downloaded: usize,
    /// A backlog of the amount of downloaded bytes each second.
    pub bytes_per_second: VecDeque<usize>,
    /// The download starting instant.
    pub start_time: Instant,
}

/// Manages progress reporting for downloads.
pub trait Progress {
    /// Invoked when a download is started.
    fn print_start(&mut self);

    /// Invoked repeatedly while a download is ongoing.
    fn print_progress(&mut self, state: &DownloadState);

    /// Invoked when a download is finished.
    fn print_finish(&mut self, state: &DownloadState);
}

/// The downloader object used for downloading packages
#[derive(Debug)]
pub struct Downloader {
    ///List of all available downloaders which can be instantiated at runtime
    http_downloader: Option<Box<dyn PackageDownloader>>,
    git_downloader: Option<Box<dyn PackageDownloader>>,
}

impl Downloader {
    /// Construct the Downloader object instantiating all the available methods.
    /// The methods can be compile-time selected by features.
    pub fn new(cert: Option<PathBuf>) -> Self {
        Self {
            http_downloader: Self::make_http_downloader(cert.clone()),
            git_downloader: Self::make_git_downloader(cert),
        }
    }

    /// Creation function for the HTTP(S) download method
    fn make_http_downloader(cert: Option<PathBuf>) -> Option<Box<dyn PackageDownloader>> {
        #[cfg(not(feature = "downloads_http"))]
        {
            None
        }

        #[cfg(feature = "downloads_http")]
        {
            match cert {
                Some(cert_path) => Some(Box::new(http::HttpDownloader::with_path(
                    http::HttpDownloader::default_user_agent(),
                    cert_path,
                ))),
                None => Some(Box::new(http::HttpDownloader::new(
                    http::HttpDownloader::default_user_agent(),
                ))),
            }
        }
    }

    fn get_http_downloader(&self) -> Result<&dyn PackageDownloader, PackageError> {
        let reference = self.http_downloader.as_ref().ok_or_else(|| {
            PackageError::Other(Some(EcoString::from(
                "Http downloader has not been initialized correctly",
            )))
        })?;
        Ok(&**reference)
    }

    /// Creation function for the GIT clone method
    fn make_git_downloader(_cert: Option<PathBuf>) -> Option<Box<dyn PackageDownloader>> {
        #[cfg(not(feature = "downloads_git"))]
        {
            None
        }

        #[cfg(feature = "downloads_git")]
        {
            Some(Box::new(git::GitDownloader::new()))
        }
    }

    fn get_git_downloader(&self) -> Result<&dyn PackageDownloader, PackageError> {
        let reference = self.git_downloader.as_ref().ok_or_else(|| {
            PackageError::Other(Some(EcoString::from(
                "Http downloader has not been initialized correctly",
            )))
        })?;
        Ok(&**reference)
    }

    /// Returns the correct downloader in function of the package namespace.
    /// The remote location of a package is encoded in its namespace in the form
    /// @<source type>:<source path>
    ///
    /// It's the downloader instance's job to parse the source path in any substructure.
    ///
    /// NOTE: Treating @preview as a special case of the https downloader.
    fn get_downloader(&self, ns: &str) -> Result<&dyn PackageDownloader, PackageError> {
        let download_type = ns.split(":").next();

        match download_type {
            #[cfg(feature = "downloads_http")]
            Some("http") | Some("https") | Some("preview") => self.get_http_downloader(),

            #[cfg(feature = "downloads_git")]
            Some("git") => self.get_git_downloader(),

            Some(dwld) => Err(PackageError::Other(Some(eco_format!(
                "Unknown downloader type: {}",
                dwld
            )))),
            None => Err(PackageError::Other(Some(EcoString::from(
                "No downloader type specified",
            )))),
        }
    }
}

impl PackageDownloader for Downloader {
    fn download_index(
        &self,
        spec: &VersionlessPackageSpec,
    ) -> Result<Vec<PackageInfo>, EcoString> {
        let downloader = self.get_downloader(spec.namespace.as_str())?;
        downloader.download_index(spec)
    }

    fn download(
        &self,
        spec: &PackageSpec,
        package_dir: &Path,
        progress: &mut dyn Progress,
    ) -> PackageResult<()> {
        let downloader = self.get_downloader(spec.namespace.as_str())?;
        downloader.download(spec, package_dir, progress)
    }
}
