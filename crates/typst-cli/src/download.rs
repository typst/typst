use std::fmt::Display;
use std::io;
use std::io::Write;
use std::time::{Duration, Instant};

use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use typst_kit::download::{DownloadState, Downloader, Progress};

use crate::terminal::{self, TermOut};
use crate::ARGS;

/// Prints download progress by writing `downloading {0}` followed by repeatedly
/// updating the last terminal line.
pub struct PrintDownload<T>(pub T);

impl<T: Display> Progress for PrintDownload<T> {
    fn print_start(&mut self) {
        // Print that a package downloading is happening.
        let styles = term::Styles::default();

        let mut out = terminal::out();
        let _ = out.set_color(&styles.header_help);
        let _ = write!(out, "downloading");

        let _ = out.reset();
        let _ = writeln!(out, " {}", self.0);
    }

    fn print_progress(&mut self, state: &DownloadState) {
        let mut out = terminal::out();
        let _ = out.clear_last_line();
        let _ = display_download_progress(&mut out, state);
    }

    fn print_finish(&mut self, state: &DownloadState) {
        let mut out = terminal::out();
        let _ = display_download_progress(&mut out, state);
        let _ = writeln!(out);
    }
}

/// Returns a new downloader.
pub fn downloader() -> Downloader {
    let user_agent = concat!("typst/", env!("CARGO_PKG_VERSION"));
    match ARGS.cert.clone() {
        Some(cert) => Downloader::with_path(user_agent, cert),
        None => Downloader::new(user_agent),
    }
}

/// Compile and format several download statistics and make and attempt at
/// displaying them on standard error.
pub fn display_download_progress(
    out: &mut TermOut,
    state: &DownloadState,
) -> io::Result<()> {
    let sum: usize = state.bytes_per_second.iter().sum();
    let len = state.bytes_per_second.len();
    let speed = if len > 0 { sum / len } else { state.content_len.unwrap_or(0) };

    let total_downloaded = as_bytes_unit(state.total_downloaded);
    let speed_h = as_throughput_unit(speed);
    let elapsed = time_suffix(Instant::now().saturating_duration_since(state.start_time));

    match state.content_len {
        Some(content_len) => {
            let percent = (state.total_downloaded as f64 / content_len as f64) * 100.;
            let remaining = content_len - state.total_downloaded;

            let download_size = as_bytes_unit(content_len);
            let eta = time_suffix(Duration::from_secs(if speed == 0 {
                0
            } else {
                (remaining / speed) as u64
            }));
            writeln!(
                out,
                "{total_downloaded} / {download_size} ({percent:3.0} %) {speed_h} in {elapsed} ETA: {eta}",
            )?;
        }
        None => writeln!(
            out,
            "Total downloaded: {total_downloaded} Speed: {speed_h} Elapsed: {elapsed}",
        )?,
    };
    Ok(())
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
