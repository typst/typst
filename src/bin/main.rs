use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process;

use typstc::export::pdf::PdfExporter;
use typstc::toddle::query::FileSystemFontProvider;
use typstc::Typesetter;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        help_and_quit();
    }

    let source_path = Path::new(&args[1]);
    let dest_path = if args.len() <= 2 {
        source_path.with_extension("pdf")
    } else {
        PathBuf::from(&args[2])
    };

    if dest_path == source_path {
        return err("source and destination path are the same");
    }

    let mut source_file = File::open(source_path)
        .map_err(|_| "failed to open source file")?;

    let mut src = String::new();
    source_file
        .read_to_string(&mut src)
        .map_err(|_| "failed to read from source file")?;

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider);

    let document = typesetter.typeset(&src)?;

    let exporter = PdfExporter::new();
    let dest_file = File::create(&dest_path)?;
    exporter.export(&document, typesetter.loader(), BufWriter::new(dest_file))?;

    Ok(())
}

/// Construct an error `Result` from a message.
fn err<S: Into<String>, T>(message: S) -> Result<T, Box<dyn Error>> {
    Err(message.into().into())
}

/// Print a usage message and exit the process.
fn help_and_quit() {
    let name = env::args().next().unwrap_or("typst".to_string());
    println!("usage: {} source [destination]", name);
    process::exit(0);
}
