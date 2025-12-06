use std::fmt::Display;
use std::io;
use std::io::Write;
use std::time::{Duration, Instant};

use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use typst::utils::format_duration;
use typst_kit::download::{DownloadState, Downloader, Progress};

use crate::ARGS;
use crate::terminal::{self, TermOut};

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
    let user_agent = format!("typst/{}", typst::utils::version().raw());
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
    let elapsed = Instant::now().saturating_duration_since(state.start_time);

    match state.content_len {
        Some(content_len) => {
            let percent = (state.total_downloaded as f64 / content_len as f64) * 100.;
            let remaining = content_len - state.total_downloaded;

            let download_size = as_bytes_unit(content_len);
            let eta = Duration::from_secs(if speed == 0 {
                0
            } else {
                (remaining / speed) as u64
            });

            writeln!(
                out,
                "{total_downloaded} / {download_size} ({percent:3.0} %) \
                {speed_h} in {elapsed} ETA: {eta}",
                elapsed = format_duration(elapsed),
                eta = format_duration(eta),
            )?;
        }
        None => writeln!(
            out,
            "Total downloaded: {total_downloaded} \
             Speed: {speed_h} \
             Elapsed: {elapsed}",
            elapsed = format_duration(elapsed),
        )?,
    };
    Ok(())
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
        format!("{size:3} B")
    }
}

fn as_throughput_unit(size: usize) -> String {
    as_bytes_unit(size) + "/s"
}
