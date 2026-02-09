use std::io::Write;

use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use ecow::{EcoString, eco_format};
use typst::syntax::package::PackageSpec;
use typst_kit::downloader::{
    Downloader, Progress, ProgressDownloader, ProgressReporter, SystemDownloader,
};

use crate::ARGS;
use crate::terminal;

/// Returns a new downloader.
pub fn downloader() -> impl Downloader {
    let user_agent = format!("typst/{}", typst::utils::version().raw());
    let system = match ARGS.cert.clone() {
        None => SystemDownloader::new(user_agent),
        Some(cert) => SystemDownloader::with_cert_path(user_agent, cert),
    };

    ProgressDownloader::new(system, |key| {
        let name = if let Some(spec) = key.downcast_ref::<PackageSpec>() {
            Some(eco_format!("{spec}"))
        } else if let Some(&s @ "release") = key.downcast_ref::<&str>() {
            Some(s.into())
        } else {
            None
        };
        PrintProgress(name)
    })
}

/// Prints download progress by writing `downloading {0}` followed by repeatedly
/// updating the last terminal line.
struct PrintProgress(Option<EcoString>);

impl ProgressReporter for PrintProgress {
    fn start(&mut self, progress: &Progress) {
        if let Some(name) = &self.0 {
            let styles = term::Styles::default();
            let mut out = terminal::out();
            _ = out.set_color(&styles.header_help);
            _ = write!(out, "downloading");
            _ = out.reset();
            _ = writeln!(out, " {name}");
            _ = writeln!(out);
        }
        self.update(progress);
    }

    fn update(&mut self, progress: &Progress) {
        if self.0.is_some() {
            let mut out = terminal::out();
            _ = out.clear_last_line();
            _ = writeln!(out, "{progress}");
        }
    }

    fn finish(&mut self, progress: &Progress) {
        self.update(progress);
    }
}
