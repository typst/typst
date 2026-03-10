//! Web requests with optional progress reporting.
//!
//! All `typst-kit` functionality that may trigger downloads goes through the
//! [`Downloader`] trait. A built-in implementation is provided through the
//! [`SystemDownloader`].
//!
//! Downloads can optionally be tracked by wrapping an existing downloader in a
//! [`ProgressDownloader`]. All downloads are identified by a dynamic key, so
//! that the progress downloader's underlying [`Progress`] reporter can decide
//! whether it wants to display something.

use std::any::Any;
use std::collections::VecDeque;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Cursor, ErrorKind, Read};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "system-downloader")]
use {
    ecow::EcoString,
    native_tls::{Certificate, TlsConnector},
    once_cell::sync::OnceCell,
    std::path::PathBuf,
};

/// Downloads resources from the network.
///
/// If the remote returns a `404` status code, the implementation should return
/// an error with [`io::ErrorKind::NotFound`].
///
/// See the module-level docs and [`ProgressDownloader`] for more information on
/// the `key` argument.
pub trait Downloader: Send + Sync + 'static {
    /// Fetches the given URL, returning an optional size hint and a reader for
    /// the remote data.
    fn stream(
        &self,
        key: &dyn Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn Read>)>;

    /// Fetches the given URL, returning the full data as a vector.
    ///
    /// This is optional to implement. A default implementation in terms of
    /// `stream` is provided.
    fn download(&self, key: &dyn Any, url: &str) -> io::Result<Vec<u8>> {
        let (hint, mut reader) = self.stream(key, url)?;
        let mut buf = match hint {
            None => Vec::new(),
            Some(size) => Vec::with_capacity(size),
        };
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

impl<T: Downloader> Downloader for Box<T> {
    fn stream(
        &self,
        key: &dyn Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        (**self).stream(key, url)
    }

    fn download(&self, key: &dyn Any, url: &str) -> io::Result<Vec<u8>> {
        (**self).download(key, url)
    }
}

impl<T: Downloader> Downloader for Arc<T> {
    fn stream(
        &self,
        key: &dyn Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        (**self).stream(key, url)
    }

    fn download(&self, key: &dyn Any, url: &str) -> io::Result<Vec<u8>> {
        (**self).download(key, url)
    }
}

/// A minimal HTTPS client for downloads.
///
/// Uses system-native TLS and respects proxying environment variables.
#[cfg(feature = "system-downloader")]
pub struct SystemDownloader {
    user_agent: EcoString,
    cert_path: Option<PathBuf>,
    cert: OnceCell<Certificate>,
}

#[cfg(feature = "system-downloader")]
impl SystemDownloader {
    /// Creates a new downloader with the given user agent and no certificate.
    pub fn new(user_agent: impl Into<EcoString>) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: None,
            cert: OnceCell::new(),
        }
    }

    /// Creates a new downloader with the given user agent and certificate.
    pub fn with_cert(user_agent: impl Into<EcoString>, cert: Certificate) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: None,
            cert: OnceCell::with_value(cert),
        }
    }

    /// Creates a new downloader with the given user agent and certificate path.
    ///
    /// If the certificate cannot be read, it is ignored.
    pub fn with_cert_path(user_agent: impl Into<EcoString>, cert_path: PathBuf) -> Self {
        Self {
            user_agent: user_agent.into(),
            cert_path: Some(cert_path),
            cert: OnceCell::new(),
        }
    }

    /// Returns the certificate this client is using, if a custom certificate is
    /// used it is loaded on first access.
    ///
    /// - Returns `None` if no certificate was configured.
    /// - Returns `Some(Ok(cert))` if the certificate was loaded successfully.
    /// - Returns `Some(Err(err))` if an error occurred while loading the
    ///   certificate.
    fn cert(&self) -> Option<io::Result<&Certificate>> {
        if let Some(cert) = self.cert.get() {
            return Some(Ok(cert));
        }

        self.cert_path.as_ref().map(|path| {
            self.cert.get_or_try_init(|| {
                let pem = std::fs::read(path)?;
                Certificate::from_pem(&pem).map_err(io::Error::other)
            })
        })
    }
}

#[cfg(feature = "system-downloader")]
impl Downloader for SystemDownloader {
    fn stream(
        &self,
        _: &dyn Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn Read>)> {
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

        let response = builder.build().get(url).call().map_err(|err| match err {
            ureq::Error::Status(404, _) => io::Error::new(io::ErrorKind::NotFound, err),
            err => io::Error::other(err),
        })?;

        let content_len: Option<usize> = response
            .header("Content-Length")
            .and_then(|header| header.parse().ok());

        Ok((content_len, response.into_reader()))
    }
}

#[cfg(feature = "system-downloader")]
impl Debug for SystemDownloader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemDownloader")
            .field("user_agent", &self.user_agent)
            .finish_non_exhaustive()
    }
}

/// Wraps a downloader and adds progress reporting to it.
///
/// Needs
/// - an underlying downloader
/// - a factory function that creates an instance of type `Progress`.
///
/// The factory function is passed an `&dyn Any` key. A key is provided for each
/// download and can be used to decide what to print. For instance, the CLI will
/// display downloads with a [`PackageSpec`](typst_syntax::package::PackageSpec)
/// key, but not with a `"package index"` key.
///
/// Keys used by functionality in `typst-kit` are documented with the respective
/// functionality.
pub struct ProgressDownloader<T, F, P>
where
    F: Fn(&dyn Any) -> P + Send + Sync + 'static,
{
    inner: T,
    progress: F,
    period: Duration,
}

impl<T, F, P> ProgressDownloader<T, F, P>
where
    T: Downloader,
    F: Fn(&dyn Any) -> P + Send + Sync + 'static,
    P: ProgressReporter + 'static,
{
    /// Creates a new progress downloader.
    pub fn new(inner: T, progress: F) -> Self {
        Self {
            inner,
            progress,
            period: Duration::from_millis(100),
        }
    }

    /// Creates a new progress downloader.
    pub fn with_interval(inner: T, progress: F, period: Duration) -> Self {
        Self { inner, progress, period }
    }
}

impl<T, F, P> Downloader for ProgressDownloader<T, F, P>
where
    T: Downloader,
    F: Fn(&dyn Any) -> P + Send + Sync + 'static,
    P: ProgressReporter + 'static,
{
    fn download(&self, key: &dyn Any, url: &str) -> io::Result<Vec<u8>> {
        let (len, reader) = self.inner.stream(key, url)?;
        let mut progress = (self.progress)(key);
        let data =
            ProgressReader::new(len, reader, self.period, &mut progress).download()?;
        Ok(data)
    }

    fn stream(
        &self,
        key: &dyn Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        let data = self.inner.download(key, url)?;
        Ok((Some(data.len()), Box::new(Cursor::new(data))))
    }
}

impl<T, F, P> Debug for ProgressDownloader<T, F, P>
where
    T: Debug,
    F: Fn(&dyn Any) -> P + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressDownloader")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

/// Manages progress reporting for downloads.
pub trait ProgressReporter {
    /// Invoked when a download is started.
    fn start(&mut self, progress: &Progress);

    /// Invoked repeatedly while a download is ongoing.
    fn update(&mut self, progress: &Progress);

    /// Invoked when a download is finished.
    fn finish(&mut self, progress: &Progress);
}

/// The current progress of a download.
#[derive(Debug)]
pub struct Progress {
    /// The download starting instant.
    pub start_time: Instant,
    /// The expected amount of bytes to download, `None` if the response header
    /// was not set.
    pub content_len: Option<usize>,
    /// The total amount of downloaded bytes until now.
    pub downloaded: usize,
    /// A backlog of the amount of downloaded bytes for each bucket.
    pub samples: VecDeque<usize>,
    /// The duration of each bucket (in samples).
    pub period: Duration,
}

impl Display for Progress {
    /// Formats several download statistics for display.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let len = self.samples.len();
        let sum: usize = self.samples.iter().sum();
        let bytes_per_period =
            if len > 0 { sum / len } else { self.content_len.unwrap_or(0) };

        let frequency: usize = Duration::from_secs(1)
            .as_nanos()
            .checked_div(self.period.as_nanos())
            .and_then(|s| s.try_into().ok())
            .unwrap_or(1);
        let bytes_per_sec = bytes_per_period * frequency;

        match self.content_len {
            Some(content_len) => {
                let ratio = self.downloaded as f64 / content_len as f64;
                let remaining_bytes = content_len - self.downloaded;
                let remaining_buckets: u32 = remaining_bytes
                    .checked_div(bytes_per_period)
                    .and_then(|c| c.try_into().ok())
                    .unwrap_or(0);

                let eta = self.period * remaining_buckets;

                write!(
                    f,
                    "{downloaded} / {total} ({percent:3.0} %), {bytes}/s, ETA: {eta}",
                    downloaded = format_byte_unit(self.downloaded),
                    total = format_byte_unit(content_len),
                    percent = 100.0 * ratio,
                    bytes = format_byte_unit(bytes_per_sec),
                    eta = format_seconds(eta),
                )
            }

            None => write!(
                f,
                "{downloaded}, {bytes}/s",
                downloaded = format_byte_unit(self.downloaded),
                bytes = format_byte_unit(bytes_per_sec),
            ),
        }
    }
}

/// Format a given size as a unit of bytes.
fn format_byte_unit(size: usize) -> impl Display {
    const KI: f64 = 1024.0;
    const MI: f64 = KI * KI;
    const GI: f64 = KI * KI * KI;

    let size = size as f64;

    typst_utils::display(move |f| {
        if size >= GI {
            write!(f, "{:5.1} GiB", size / GI)
        } else if size >= MI {
            write!(f, "{:5.1} MiB", size / MI)
        } else if size >= KI {
            write!(f, "{:5.1} KiB", size / KI)
        } else {
            write!(f, "{size:3} B")
        }
    })
}

/// Formats a duration with second precision.
fn format_seconds(duration: Duration) -> impl Display {
    typst_utils::display(move |f| write!(f, "{} s", duration.as_secs()))
}

// Acknowledgement:
// The `RemoteReader` is closely modelled after rustup's `DownloadTracker`.
// https://github.com/rust-lang/rustup/blob/master/src/cli/download_tracker.rs

/// Keep track of this many download speed samples.
const SAMPLES: usize = 25;

/// A wrapper around [`ureq::Response`] that reads the response body in chunks
/// over a websocket and reports its progress.
struct ProgressReader<'p> {
    /// The reader returned by the ureq::Response.
    reader: Box<dyn Read>,
    /// The download state, holding download metadata for progress reporting.
    state: Progress,
    /// The instant at which progress was last reported.
    last_progress: Option<Instant>,
    /// A trait object used to report download progress.
    progress: &'p mut dyn ProgressReporter,
}

impl<'p> ProgressReader<'p> {
    /// Wraps a [`ureq::Response`] and prepares it for downloading.
    ///
    /// The 'Content-Length' header is used as a size hint for read
    /// optimization, if present.
    fn new(
        content_len: Option<usize>,
        reader: Box<dyn Read>,
        period: Duration,
        progress: &'p mut dyn ProgressReporter,
    ) -> Self {
        Self {
            reader,
            last_progress: None,
            state: Progress {
                content_len,
                downloaded: 0,
                samples: VecDeque::with_capacity(SAMPLES),
                start_time: Instant::now(),
                period,
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

        self.progress.start(&self.state);

        let mut downloaded_this_period = 0;
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

            downloaded_this_period += read;
            self.state.downloaded += read;

            if elapsed >= self.state.period {
                if self.state.samples.len() == SAMPLES {
                    self.state.samples.pop_back();
                }

                self.state.samples.push_front(downloaded_this_period);
                downloaded_this_period = 0;

                self.progress.update(&self.state);
                self.last_progress = Some(Instant::now());
            }
        }

        self.progress.finish(&self.state);
        Ok(data)
    }
}
