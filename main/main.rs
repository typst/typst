use std::cell::RefCell;
use std::fs::{read_to_string, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsProvider};
use fontdock::FontLoader;
use futures_executor::block_on;

use typstc::export::pdf;
use typstc::font::DynProvider;
use typstc::Typesetter;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("Usage: typst src.typ [out.pdf]");
        return;
    }

    let src_path = Path::new(&args[1]);
    let dest_path = if args.len() <= 2 {
        src_path.with_extension("pdf")
    } else {
        PathBuf::from(&args[2])
    };

    if src_path == dest_path {
        panic!("source and destination path are the same");
    }

    let src = read_to_string(src_path)
        .expect("failed to read from source file");

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

    let file = File::create(&dest_path)
        .expect("failed to create output file");

    let writer = BufWriter::new(file);
    pdf::export(&layouts, &loader, writer)
        .expect("failed to export pdf");
}
