use std::cell::RefCell;
use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use futures_executor::block_on;

use fontdock::fs::{FsIndex, FsProvider};
use fontdock::FontLoader;
use typstc::Typesetter;
use typstc::font::DynProvider;
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
        println!("Usage: typst src.typ [out.pdf]");
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

    let mut index = FsIndex::new();
    index.search_dir("fonts");
    index.search_os();

    let (descriptors, files) = index.into_vecs();
    let provider = FsProvider::new(files.clone());
    let dynamic = Box::new(provider) as Box<DynProvider>;
    let loader = FontLoader::new(dynamic, descriptors);
    let loader = Rc::new(RefCell::new(loader));

    let typesetter = Typesetter::new(loader.clone());
    let layouts = block_on(typesetter.typeset(&src)).output;

    let writer = BufWriter::new(File::create(&dest)?);
    pdf::export(&layouts, &loader, writer)?;

    Ok(())
}
