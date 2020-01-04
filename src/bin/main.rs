use std::fs::{File, read_to_string};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use typstc::Typesetter;
use typstc::toddle::query::FileSystemFontProvider;
use typstc::export::pdf::PdfExporter;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("usage: {} source [destination]",
            args.first().map(|s| s.as_str()).unwrap_or("typst"));
        std::process::exit(0);
    }

    let source = Path::new(&args[1]);
    let dest = if args.len() <= 2 {
        source.with_extension("pdf")
    } else {
        PathBuf::from(&args[2])
    };

    if source == dest {
        Err("source and destination path are the same")?;
    }

    let src = read_to_string(source)
        .map_err(|_| "failed to read from source file")?;

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_index("../fonts/index.json").unwrap();
    typesetter.add_font_provider(provider);

    let layouts = typesetter.typeset(&src)?;

    let exporter = PdfExporter::new();
    let writer = BufWriter::new(File::create(&dest)?);
    exporter.export(&layouts, typesetter.loader(), writer)?;

    Ok(())
}
