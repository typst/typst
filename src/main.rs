use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{anyhow, bail, Context};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::{self, Files};
use codespan_reporting::term::{self, termcolor, Config, Styles};
use same_file::is_same_file;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use typst::diag::{Error, Tracepoint};
use typst::loading::{FileId, FsLoader};
use typst::source::{SourceFile, SourceMap};

fn main() {
    if let Err(error) = try_main() {
        print_error(error).unwrap();
        process::exit(1);
    }
}

/// The main compiler logic.
fn try_main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        print_usage().unwrap();
        process::exit(2);
    }

    // Determine source and destination path.
    let src_path = Path::new(&args[1]);
    let dest_path = if let Some(arg) = args.get(2) {
        PathBuf::from(arg)
    } else {
        let name = src_path
            .file_name()
            .ok_or_else(|| anyhow!("source path is not a file"))?;

        Path::new(name).with_extension("pdf")
    };

    // Ensure that the source file is not overwritten.
    if is_same_file(src_path, &dest_path).unwrap_or(false) {
        bail!("source and destination files are the same");
    }

    // Create a loader for fonts and files.
    let loader = typst::loading::FsLoader::new()
        .with_path("fonts")
        .with_system()
        .wrap();

    // Resolve the file id of the source file and read the file.
    let file = loader.resolve(src_path).context("source file not found")?;
    let string = fs::read_to_string(&src_path).context("failed to read source file")?;
    let source = SourceFile::new(file, string);

    // Typeset.
    let mut ctx = typst::Context::new(loader.clone());
    match ctx.typeset(&source) {
        // Export the PDF.
        Ok(document) => {
            let buffer = typst::export::pdf(&ctx, &document);
            fs::write(&dest_path, buffer).context("failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            ctx.sources.insert(source);
            print_diagnostics(&loader, &ctx.sources, *errors)
                .context("failed to print diagnostics")?;
        }
    }

    Ok(())
}

/// Print a usage message.
fn print_usage() -> io::Result<()> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    writer.set_color(&styles.header_help)?;
    write!(writer, "usage")?;

    writer.set_color(&styles.header_message)?;
    writeln!(writer, ": typst document.typ [output.pdf]")?;

    writer.reset()
}

/// Print an error outside of a source file.
fn print_error(error: anyhow::Error) -> io::Result<()> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    for (i, cause) in error.chain().enumerate() {
        writer.set_color(&styles.header_error)?;
        write!(writer, "{}", if i == 0 { "error" } else { "cause" })?;

        writer.set_color(&styles.header_message)?;
        writeln!(writer, ": {}", cause)?;
    }

    writer.reset()
}

/// Print diagnostics messages to the terminal.
fn print_diagnostics(
    loader: &FsLoader,
    sources: &SourceMap,
    errors: Vec<Error>,
) -> Result<(), files::Error> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let config = Config { tab_width: 2, ..Default::default() };
    let files = FilesImpl(loader, sources);

    for error in errors {
        // The main diagnostic.
        let main = Diagnostic::error()
            .with_message(error.message)
            .with_labels(vec![Label::primary(error.file, error.span.to_range())]);

        term::emit(&mut writer, &config, &files, &main)?;

        // Stacktrace-like helper diagnostics.
        for (file, span, point) in error.trace {
            let message = match point {
                Tracepoint::Call(Some(name)) => {
                    format!("error occured in this call of function `{}`", name)
                }
                Tracepoint::Call(None) => "error occured in this function call".into(),
                Tracepoint::Import => "error occured while importing this module".into(),
            };

            let help = Diagnostic::help()
                .with_message(message)
                .with_labels(vec![Label::primary(file, span.to_range())]);

            term::emit(&mut writer, &config, &files, &help)?;
        }
    }

    Ok(())
}

/// Required for error message formatting with codespan-reporting.
struct FilesImpl<'a>(&'a FsLoader, &'a SourceMap);

impl FilesImpl<'_> {
    fn source(&self, id: FileId) -> Result<&SourceFile, files::Error> {
        self.1.get(id).ok_or(files::Error::FileMissing)
    }
}

impl<'a> Files<'a> for FilesImpl<'a> {
    type FileId = FileId;
    type Name = String;
    type Source = &'a str;

    fn name(&'a self, id: FileId) -> Result<Self::Name, files::Error> {
        Ok(self.0.path(id).display().to_string())
    }

    fn source(&'a self, id: FileId) -> Result<Self::Source, files::Error> {
        Ok(self.source(id)?.src())
    }

    fn line_index(
        &'a self,
        id: FileId,
        byte_index: usize,
    ) -> Result<usize, files::Error> {
        let source = self.source(id)?;
        source.pos_to_line(byte_index.into()).ok_or_else(|| {
            let (given, max) = (byte_index, source.len_bytes());
            if given <= max {
                files::Error::InvalidCharBoundary { given }
            } else {
                files::Error::IndexTooLarge { given, max }
            }
        })
    }

    fn line_range(
        &'a self,
        id: FileId,
        line_index: usize,
    ) -> Result<Range<usize>, files::Error> {
        let source = self.source(id)?;
        let span = source.line_to_span(line_index).ok_or(files::Error::LineTooLarge {
            given: line_index,
            max: source.len_lines(),
        })?;
        Ok(span.to_range())
    }
}
