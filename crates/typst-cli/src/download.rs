// Acknowledgement:
// Closely modelled after rustup's `DownloadTracker`.
// https://github.com/rust-lang/rustup/blob/master/src/cli/download_tracker.rs

use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use native_tls::{Certificate, TlsConnector};
use once_cell::sync::Lazy;
use ureq::Response;

use crate::terminal;

/// Keep track of this many download speed samples.
const SPEED_SAMPLES: usize = 5;

/// Lazily loads a custom CA certificate if present, but if there's an error
/// loading certificate, it just uses the default configuration.
static CERT: Lazy<Option<Certificate>> = Lazy::new(|| {
    let path = crate::ARGS.cert.as_ref()?;
    let pem = std::fs::read(path).ok()?;
    Certificate::from_pem(&pem).ok()
});

/// Download binary data and display its progress.
#[allow(clippy::result_large_err)]
pub fn download_with_progress(url: &str) -> Result<Vec<u8>, ureq::Error> {
    let response = download(url)?;
    Ok(RemoteReader::from_response(response).download()?)
}

/// Download from a URL.
#[allow(clippy::result_large_err)]
pub fn download(url: &str) -> Result<ureq::Response, ureq::Error> {
    let mut builder = ureq::AgentBuilder::new();
    let mut tls = TlsConnector::builder();

    // Set user agent.
    builder = builder.user_agent(concat!("typst/", env!("CARGO_PKG_VERSION")));

    // Get the network proxy config from the environment and apply it.
    if let Some(proxy) = env_proxy::for_url_str(url)
        .to_url()
        .and_then(|url| ureq::Proxy::new(url).ok())
    {
        builder = builder.proxy(proxy);
    }

    // Apply a custom CA certificate if present.
    if let Some(cert) = &*CERT {
        tls.add_root_certificate(cert.clone());
    }

    // Configure native TLS.
    let connector =
        tls.build().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    builder = builder.tls_connector(Arc::new(connector));

    builder.build().get(url).call()
}

/// A wrapper around [`ureq::Response`] that reads the response body in chunks
/// over a websocket and displays statistics about its progress.
///
/// Downloads will _never_ fail due to statistics failing to print, print errors
/// are silently ignored.
struct RemoteReader {
    reader: Box<dyn Read + Send + Sync + 'static>,
    content_len: Option<usize>,
    total_downloaded: usize,
    downloaded_this_sec: usize,
    downloaded_last_few_secs: VecDeque<usize>,
    start_time: Instant,
    last_print: Option<Instant>,
}

impl RemoteReader {
    /// Wraps a [`ureq::Response`] and prepares it for downloading.
    ///
    /// The 'Content-Length' header is used as a size hint for read
    /// optimization, if present.
    pub fn from_response(response: Response) -> Self {
        let content_len: Option<usize> = response
            .header("Content-Length")
            .and_then(|header| header.parse().ok());

        Self {
            reader: response.into_reader(),
            content_len,
            total_downloaded: 0,
            downloaded_this_sec: 0,
            downloaded_last_few_secs: VecDeque::with_capacity(SPEED_SAMPLES),
            start_time: Instant::now(),
            last_print: None,
        }
    }

    /// Download the bodies content as raw bytes while attempting to print
    /// download statistics to standard error. Download progress gets displayed
    /// and updated every second.
    ///
    /// These statistics will never prevent a download from completing, errors
    /// are silently ignored.
    pub fn download(mut self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; 8192];
        let mut data = match self.content_len {
            Some(content_len) => Vec::with_capacity(content_len),
            None => Vec::with_capacity(8192),
        };

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

            let last_printed = match self.last_print {
                Some(prev) => prev,
                None => {
                    let current_time = Instant::now();
                    self.last_print = Some(current_time);
                    current_time
                }
            };
            let elapsed = Instant::now().saturating_duration_since(last_printed);

            self.total_downloaded += read;
            self.downloaded_this_sec += read;

            if elapsed >= Duration::from_secs(1) {
                if self.downloaded_last_few_secs.len() == SPEED_SAMPLES {
                    self.downloaded_last_few_secs.pop_back();
                }

                self.downloaded_last_few_secs.push_front(self.downloaded_this_sec);
                self.downloaded_this_sec = 0;

                terminal::out().clear_last_line()?;
                self.display()?;
                self.last_print = Some(Instant::now());
            }
        }

        self.display()?;
        writeln!(&mut terminal::out())?;

        Ok(data)
    }

    /// Compile and format several download statistics and make an attempt at
    /// displaying them on standard error.
    fn display(&mut self) -> io::Result<()> {
        let sum: usize = self.downloaded_last_few_secs.iter().sum();
        let len = self.downloaded_last_few_secs.len();
        let speed = if len > 0 { sum / len } else { self.content_len.unwrap_or(0) };

        let total_downloaded = as_bytes_unit(self.total_downloaded);
        let speed_h = as_throughput_unit(speed);
        let elapsed =
            time_suffix(Instant::now().saturating_duration_since(self.start_time));

        match self.content_len {
            Some(content_len) => {
                let percent = (self.total_downloaded as f64 / content_len as f64) * 100.;
                let remaining = content_len - self.total_downloaded;

                let download_size = as_bytes_unit(content_len);
                let eta = time_suffix(Duration::from_secs(if speed == 0 {
                    0
                } else {
                    (remaining / speed) as u64
                }));
                writeln!(
                    terminal::out(),
                    "{total_downloaded} / {download_size} ({percent:3.0} %) {speed_h} in {elapsed} ETA: {eta}",
                )?;
            }
            None => writeln!(
                terminal::out(),
                "Total downloaded: {total_downloaded} Speed: {speed_h} Elapsed: {elapsed}",
            )?,
        };
        Ok(())
    }
}

/// Append a unit-of-time suffix.
fn time_suffix(duration: Duration) -> String {
    let secs = duration.as_secs();
    match format_dhms(secs) {
        (0, 0, 0, s) => format!("{s:2.0}s"),
        (0, 0, m, s) => format!("{m:2.0}m {s:2.0}s"),
        (0, h, m, s) => format!("{h:2.0}h {m:2.0}m {s:2.0}s"),
        (d, h, m, s) => format!("{d:3.0}d {h:2.0}h {m:2.0}m {s:2.0}s"),
    }
}

/// Format the total amount of seconds into the amount of days, hours, minutes
/// and seconds.
fn format_dhms(sec: u64) -> (u64, u8, u8, u8) {
    let (mins, sec) = (sec / 60, (sec % 60) as u8);
    let (hours, mins) = (mins / 60, (mins % 60) as u8);
    let (days, hours) = (hours / 24, (hours % 24) as u8);
    (days, hours, mins, sec)
}

/// Format a given size as a unit of time. Setting `include_suffix` to true
/// appends a '/s' (per second) suffix.
fn as_bytes_unit(size: usize) -> String {
    const KI: f64 = 1024.0;
    const MI: f64 = KI * KI;
    const GI: f64 = KI * KI * KI;

    let size = size as f64;

    if size >= GI {
        format!("{:5.1} GiB", size / GI)
    } else if size >= MI {
        format!("{:5.1} MiB", size / MI)
    } else if size >= KI {
        format!("{:5.1} KiB", size / KI)
    } else {
        format!("{size:3.0} B")
    }
}

fn as_throughput_unit(size: usize) -> String {
    as_bytes_unit(size) + "/s"
}
