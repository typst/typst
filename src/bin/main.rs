use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use futures_executor::block_on;

use typstc::{Typesetter, DebugErrorProvider};
use typstc::toddle::query::fs::EagerFsProvider;
use typstc::export::pdf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
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

    let (fs, entries) = EagerFsProvider::from_index("../fonts", "index.json")?;
    let provider = DebugErrorProvider::new(fs);
    let typesetter = Typesetter::new((Box::new(provider), entries));

    let layouts = block_on(typesetter.typeset(&src)).output;

    let writer = BufWriter::new(File::create(&dest)?);
    pdf::export(&layouts, typesetter.loader(), writer)?;

    Ok(())
}
