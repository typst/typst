// Acknowledgement:
// Closely modelled after rustup's `DownloadTracker`.
// https://github.com/rust-lang/rustup/blob/master/src/cli/download_tracker.rs

//! Helpers for making various web requests with status reporting. These are
//! primarily used for communicating with package registries.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{self, ErrorKind, Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ecow::EcoString;
use native_tls::{Certificate, TlsConnector};
use once_cell::sync::OnceCell;
use ureq::Response;

/// Manages progress reporting for downloads.
pub trait Progress {
    /// Invoked when a download is started.
    fn print_start(&mut self);

    /// Invoked repeatedly while a download is ongoing.
    fn print_progress(&mut self, state: &DownloadState);

    /// Invoked when a download is finished.
    fn print_finish(&mut self, state: &DownloadState);
}

/// An implementation of [`Progress`] with no-op reporting, i.e., reporting
/// events are swallowed.
pub struct ProgressSink;

impl Progress for ProgressSink {
    fn print_start(&mut self) {}
    fn print_progress(&mut self, _: &DownloadState) {}
    fn print_finish(&mut self, _: &DownloadState) {}
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

/// A minimal https client for downloading various resources.
pub struct Downloader {
    user_agent: EcoString,
    cert_path: Option<PathBuf>,
    cert: OnceCell<Certificate>,
}

impl Downloader {
    /// Crates a new downloader with the given user agent and no certificate.
    pub fn new(user_agent: impl Into<EcoString>) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: None,
            cert: OnceCell::new(),
        }
    }

    /// Crates a new downloader with the given user agent and certificate path.
    ///
    /// If the certificate cannot be read it is set to `None`.
    pub fn with_path(user_agent: impl Into<EcoString>, cert_path: PathBuf) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: Some(cert_path),
            cert: OnceCell::new(),
        }
    }

    /// Crates a new downloader with the given user agent and certificate.
    pub fn with_cert(user_agent: impl Into<EcoString>, cert: Certificate) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: None,
            cert: OnceCell::with_value(cert),
        }
    }

    /// Returns the certificate this client is using, if a custom certificate
    /// is used it is loaded on first access.
    ///
    /// - Returns `None` if `--cert` and `TYPST_CERT` are not set.
    /// - Returns `Some(Ok(cert))` if the certificate was loaded successfully.
    /// - Returns `Some(Err(err))` if an error occurred while loading the certificate.
    pub fn cert(&self) -> Option<io::Result<&Certificate>> {
        self.cert_path.as_ref().map(|path| {
            self.cert.get_or_try_init(|| {
                let pem = std::fs::read(path)?;
                Certificate::from_pem(&pem).map_err(io::Error::other)
            })
        })
    }

    /// Download binary data from the given url.
    #[allow(clippy::result_large_err)]
    pub fn download(&self, url: &str) -> Result<ureq::Response, ureq::Error> {
        let mut builder = ureq::AgentBuilder::new();
        let mut tls = TlsConnector::builder();

        // Set user agent.
        builder = builder.user_agent(&self.user_agent);

        // Get the network proxy config from the environment and apply it.
        if let Some(proxy) = env_proxy::for_url_str(url)
            .to_url()
            .and_then(|url| ureq::Proxy::new(url).ok())
        {
            builder = builder.proxy(proxy);
        }

        // Apply a custom CA certificate if present.
        if let Some(cert) = self.cert() {
            tls.add_root_certificate(cert?.clone());
        }

        // Configure native TLS.
        let connector = tls.build().map_err(io::Error::other)?;
        builder = builder.tls_connector(Arc::new(connector));

        builder.build().get(url).call()
    }

    /// Download binary data from the given url and report its progress.
    #[allow(clippy::result_large_err)]
    pub fn download_with_progress(
        &self,
        url: &str,
        progress: &mut dyn Progress,
    ) -> Result<Vec<u8>, ureq::Error> {
        progress.print_start();
        let response = self.download(url)?;
        Ok(RemoteReader::from_response(response, progress).download()?)
    }
}

impl Debug for Downloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Downloader")
            .field("user_agent", &self.user_agent)
            .field("cert_path", &self.cert_path)
            .field(
                "cert",
                &self
                    .cert
                    .get()
                    .map(|_| typst_utils::debug(|f| write!(f, "Certificate(..)"))),
            )
            .finish()
    }
}

/// Keep track of this many download speed samples.
const SAMPLES: usize = 5;

/// A wrapper around [`ureq::Response`] that reads the response body in chunks
/// over a websocket and reports its progress.
struct RemoteReader<'p> {
    /// The reader returned by the ureq::Response.
    reader: Box<dyn Read + Send + Sync + 'static>,
    /// The download state, holding download metadata for progress reporting.
    state: DownloadState,
    /// The instant at which progress was last reported.
    last_progress: Option<Instant>,
    /// A trait object used to report download progress.
    progress: &'p mut dyn Progress,
}

impl<'p> RemoteReader<'p> {
    /// Wraps a [`ureq::Response`] and prepares it for downloading.
    ///
    /// The 'Content-Length' header is used as a size hint for read
    /// optimization, if present.
    fn from_response(response: Response, progress: &'p mut dyn Progress) -> Self {
        let content_len: Option<usize> = response
            .header("Content-Length")
            .and_then(|header| header.parse().ok());

        Self {
            reader: response.into_reader(),
            last_progress: None,
            state: DownloadState {
                content_len,
                total_downloaded: 0,
                bytes_per_second: VecDeque::with_capacity(SAMPLES),
                start_time: Instant::now(),
            },
            progress,
        }
    }

    /// Download the body's content as raw bytes while reporting download
    /// progress.
    fn download(mut self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; 8192];
        let mut data = match self.state.content_len {
            Some(content_len) => Vec::with_capacity(content_len),
            None => Vec::with_capacity(8192),
        };

        let mut downloaded_this_sec = 0;
        loop {
            let read = match self.reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                // If the data is not yet ready but will be available eventually
                // keep trying until we either get an actual error, receive data
                // or an Ok(0).
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };

            data.extend(&buffer[..read]);

            let last_printed = match self.last_progress {
                Some(prev) => prev,
                None => {
                    let current_time = Instant::now();
                    self.last_progress = Some(current_time);
                    current_time
                }
            };
            let elapsed = Instant::now().saturating_duration_since(last_printed);

            downloaded_this_sec += read;
            self.state.total_downloaded += read;

            if elapsed >= Duration::from_secs(1) {
                if self.state.bytes_per_second.len() == SAMPLES {
                    self.state.bytes_per_second.pop_back();
                }

                self.state.bytes_per_second.push_front(downloaded_this_sec);
                downloaded_this_sec = 0;

                self.progress.print_progress(&self.state);
                self.last_progress = Some(Instant::now());
            }
        }

        self.progress.print_finish(&self.state);

        Ok(data)
    }
}
