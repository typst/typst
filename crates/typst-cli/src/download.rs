use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Stdout, Write};
use std::time::{Duration, Instant};

use ureq::Response;

pub struct RemoteReader {
    reader: Box<dyn Read + Send + Sync + 'static>,
    content_len: usize,
    total_downloaded: usize,
    downloaded_this_sec: usize,
    downloaded_last_few_secs: VecDeque<usize>,
    start_time: Instant,
    last_print: Option<Instant>,
    displayed_charcount: Option<usize>,
    terminal: Stdout,
}

impl RemoteReader {
    pub fn from_response(response: Response) -> Self {
        let content_len = response
            .header("Content-Length")
            .and_then(|header| header.parse().ok())
            .unwrap_or_default();

        Self {
            reader: response.into_reader(),
            content_len,
            total_downloaded: 0,
            downloaded_this_sec: 0,
            downloaded_last_few_secs: VecDeque::new(),
            start_time: Instant::now(),
            last_print: None,
            displayed_charcount: None,
            terminal: std::io::stdout(),
        }
    }

    pub fn download(mut self) -> io::Result<Vec<u8>> {
        if self.content_len == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        let mut data = vec![0; self.content_len];
        let mut offset = 0;

        loop {
            let read = match self.reader.read(&mut data[offset..]) {
                Ok(0) => break,
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            offset += read;

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
                self.downloaded_last_few_secs.push_front(self.downloaded_this_sec);
                self.downloaded_this_sec = 0;

                if let Some(n) = self.displayed_charcount {
                    self.erase_chars(n);
                }

                self.display();
                let _ = write!(self.terminal, "\r");
                self.last_print = Some(Instant::now());
            }

            if read == 0 {
                assert_eq!(self.total_downloaded, self.content_len);
                break;
            }
        }

        self.display();
        let _ = writeln!(self.terminal);

        assert_eq!(self.total_downloaded, self.content_len);

        Ok(data)
    }

    fn display(&mut self) {
        let percent = (self.total_downloaded as f64 / self.content_len as f64) * 100.;
        let sum: usize = self.downloaded_last_few_secs.iter().sum();
        let len = self.downloaded_last_few_secs.len();
        let speed = if len > 0 { sum / len } else { self.content_len };
        let remaining = self.content_len - self.total_downloaded;

        let output = format!(
            "{} / {} ({:3.0} %) {} in {} ETA: {}",
            format_speed(self.total_downloaded, false),
            format_speed(self.content_len, false),
            percent,
            format_speed(speed, true),
            time_suffix(Instant::now().saturating_duration_since(self.start_time)),
            time_suffix(Duration::from_secs(if speed == 0 {
                0
            } else {
                (remaining / speed) as u64
            }))
        );

        let _ = write!(self.terminal, "{output}");
        let _ = self.terminal.flush();

        self.displayed_charcount = Some(output.chars().count());
    }

    fn erase_chars(&mut self, count: usize) {
        let _ = write!(self.terminal, "{}", " ".repeat(count));
        let _ = self.terminal.flush();
        let _ = write!(self.terminal, "\r");
    }
}

fn time_suffix(duration: Duration) -> String {
    let secs = duration.as_secs();
    match format_dhms(secs) {
        (0, 0, 0, s) => format!("{s:2.0}s"),
        (0, 0, m, s) => format!("{m:2.0}m {s:2.0}s"),
        (0, h, m, s) => format!("{h:2.0}h {m:2.0}m {s:2.0}s"),
        (d, h, m, s) => format!("{d:3.0}d {h:2.0}h {m:2.0}m {s:2.0}s"),
    }
}

fn format_dhms(sec: u64) -> (u64, u8, u8, u8) {
    let (mins, sec) = (sec / 60, (sec % 60) as u8);
    let (hours, mins) = (mins / 60, (mins % 60) as u8);
    let (days, hours) = (hours / 24, (hours % 24) as u8);
    (days, hours, mins, sec)
}

fn format_speed(size: usize, include_suffix: bool) -> String {
    const KI: f64 = 1024.0;
    const MI: f64 = KI * KI;
    const GI: f64 = KI * KI * KI;

    let size = size as f64;

    let suffix = if include_suffix { "/s" } else { "" };

    if size >= GI {
        format!("{:5.1} GiB{}", size / GI, suffix)
    } else if size >= MI {
        format!("{:5.1} MiB{}", size / MI, suffix)
    } else if size >= KI {
        format!("{:5.1} KiB{}", size / KI, suffix)
    } else {
        format!("{size:3.0} B{}", suffix)
    }
}
