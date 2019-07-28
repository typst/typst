use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, BufWriter};
use std::path::{Path, PathBuf};
use std::process;

use typeset::Typesetter;
use typeset::font::FileSystemFontProvider;
use typeset::export::pdf::PdfExporter;


fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

/// The actual main function.
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        help_and_quit();
    }

    let source_path = Path::new(&args[1]);

    // Compute the output filename from the input filename by replacing the extension.
    let dest_path = if args.len() <= 2 {
        let stem = source_path.file_stem().ok_or_else(|| "missing destation file name")?;
        let base = source_path.parent().ok_or_else(|| "missing destation folder")?;
        base.join(format!("{}.pdf", stem.to_string_lossy()))
    } else {
        PathBuf::from(&args[2])
    };

    if dest_path == source_path {
        return Err("source and destination path are the same".into());
    }

    let mut src = String::new();
    let mut source_file = File::open(source_path).map_err(|_| "failed to open source file")?;
    source_file.read_to_string(&mut src).map_err(|_| "failed to read from source file")?;

    // Create a typesetter with a font provider that provides the default fonts.
    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider);

    // Typeset the source code.
    let document = typesetter.typeset(&src)?;

    // Export the document into a PDF file.
    let exporter = PdfExporter::new();
    let dest_file = File::create(&dest_path)?;
    exporter.export(&document, BufWriter::new(dest_file))?;

    Ok(())
}

/// Print a usage message and quit.
fn help_and_quit() {
    let name = env::args().next().unwrap_or("typst".to_string());
    println!("usage: {} source [destination]", name);
    process::exit(0);
}
