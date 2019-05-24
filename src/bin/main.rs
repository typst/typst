use std::env;
use std::fs::File;
use std::error::Error;
use std::process;
use std::io::Read;
use std::path::{Path, PathBuf};

use typeset::Typesetter;
use typeset::{font::FileSystemFontProvider, font_info};
use typeset::export::pdf::PdfExporter;


fn run() -> Result<(), Box<Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        help_and_quit();
    }

    // Open the input file.
    let mut file = File::open(&args[1]).map_err(|_| "failed to open source file")?;

    // The output file name.
    let output_filename = if args.len() <= 2 {
        let source_path = Path::new(&args[1]);
        let stem = source_path.file_stem().ok_or_else(|| "missing destation file name")?;
        let base = source_path.parent().ok_or_else(|| "missing destation folder")?;
        base.join(format!("{}.pdf", stem.to_string_lossy()))
    } else {
        PathBuf::from(&args[2])
    };

    // Read the input file.
    let mut src = String::new();
    file.read_to_string(&mut src).map_err(|_| "failed to read from source file")?;

    // Create a typesetter with a font provider that provides three fonts
    // (two sans-serif fonts and a fallback for the emoji).
    let mut typesetter = Typesetter::new();
    typesetter.add_font_provider(FileSystemFontProvider::new("fonts", vec![
        ("NotoSans-Regular.ttf",     font_info!(["NotoSans", "Noto", SansSerif])),
        ("NotoSans-Italic.ttf",      font_info!(["NotoSans", "Noto", SansSerif], italic)),
        ("NotoSans-Bold.ttf",        font_info!(["NotoSans", "Noto", SansSerif], bold)),
        ("NotoSans-BoldItalic.ttf",  font_info!(["NotoSans", "Noto", SansSerif], italic, bold)),
        ("NotoSansMath-Regular.ttf", font_info!(["NotoSansMath", "Noto", SansSerif])),
        ("NotoEmoji-Regular.ttf",    font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
    ]));

    // Typeset the source code.
    let document = typesetter.typeset(&src)?;

    // Export the document into a PDF file.
    let exporter = PdfExporter::new();
    let output_file = File::create(&output_filename)?;
    exporter.export(&document, output_file)?;

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

/// Print a usage message and quit.
fn help_and_quit() {
    let name = env::args().next().unwrap_or("help".to_string());
    println!("usage: {} <source> [<destination>]", name);
    process::exit(0);
}
