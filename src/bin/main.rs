use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

use typeset::Typesetter;
use typeset::{font::FileSystemFontProvider, font_info};
use typeset::export::pdf::PdfExporter;


fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

/// The actual main function.
fn run() -> Result<(), Box<Error>> {
    // Check the command line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        help_and_quit();
    }

    // Open the input file.
    let mut file = File::open(&args[1]).map_err(|_| "failed to open source file")?;

    let source_path = Path::new(&args[1]);

    // Compute the output filename from the input filename by replacing the extension.
    let dest_path = if args.len() <= 2 {
        let stem = source_path.file_stem().ok_or_else(|| "missing destation file name")?;
        let base = source_path.parent().ok_or_else(|| "missing destation folder")?;
        base.join(format!("{}.pdf", stem.to_string_lossy()))
    } else {
        PathBuf::from(&args[2])
    };

    // We do not want to overwrite the source file.
    if dest_path == source_path {
        return Err("source and destination path are the same".into());
    }

    // Read the input file.
    let mut src = String::new();
    file.read_to_string(&mut src).map_err(|_| "failed to read from source file")?;

    // Create a typesetter with a font provider that provides the default fonts.
    let mut typesetter = Typesetter::new();
    typesetter.add_font_provider(FileSystemFontProvider::new("fonts", vec![
        ("CMU-SansSerif-Regular.ttf", font_info!(["Computer Modern", SansSerif])),
        ("CMU-SansSerif-Italic.ttf", font_info!(["Computer Modern", SansSerif], italic)),
        ("CMU-SansSerif-Bold.ttf", font_info!(["Computer Modern", SansSerif], bold)),
        ("CMU-SansSerif-Bold-Italic.ttf", font_info!(["Computer Modern", SansSerif], bold, italic)),
        ("CMU-Serif-Regular.ttf", font_info!(["Computer Modern", Serif])),
        ("CMU-Serif-Italic.ttf", font_info!(["Computer Modern", Serif], italic)),
        ("CMU-Serif-Bold.ttf", font_info!(["Computer Modern", Serif], bold)),
        ("CMU-Serif-Bold-Italic.ttf", font_info!(["Computer Modern", Serif], bold, italic)),
        ("CMU-Typewriter-Regular.ttf", font_info!(["Computer Modern", Monospace])),
        ("CMU-Typewriter-Italic.ttf", font_info!(["Computer Modern", Monospace], italic)),
        ("CMU-Typewriter-Bold.ttf", font_info!(["Computer Modern", Monospace], bold)),
        ("CMU-Typewriter-Bold-Italic.ttf", font_info!(["Computer Modern", Monospace], bold, italic)),
        ("NotoEmoji-Regular.ttf", font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
    ]));

    // Typeset the source code.
    let document = typesetter.typeset(&src)?;

    // Export the document into a PDF file.
    let exporter = PdfExporter::new();
    let output_file = File::create(&dest_path)?;
    exporter.export(&document, output_file)?;

    Ok(())
}

/// Print a usage message and quit.
fn help_and_quit() {
    let name = env::args().next().unwrap_or("typeset".to_string());
    println!("usage: {} source [destination]", name);
    process::exit(0);
}
