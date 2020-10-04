use std::cell::RefCell;
use std::fs::{read_to_string, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsProvider};
use futures_executor::block_on;

use typstc::export::pdf;
use typstc::font::FontLoader;
use typstc::parse::LineMap;
use typstc::{typeset, Feedback, Pass};

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

    let src = read_to_string(src_path).expect("failed to read from source file");

    let mut index = FsIndex::new();
    index.search_dir("fonts");
    index.search_os();

    let (descriptors, files) = index.into_vecs();
    let provider = FsProvider::new(files);
    let loader = FontLoader::new(Box::new(provider), descriptors);
    let loader = Rc::new(RefCell::new(loader));

    let style = Default::default();
    let scope = typstc::library::_std();
    let Pass {
        output: layouts,
        feedback: Feedback { mut diagnostics, .. },
    } = block_on(typeset(&src, &style, &scope, Rc::clone(&loader)));

    if !diagnostics.is_empty() {
        diagnostics.sort();

        let map = LineMap::new(&src);
        for diagnostic in diagnostics {
            let span = diagnostic.span;
            let start = map.location(span.start);
            let end = map.location(span.end);

            println!(
                "  {}: {}:{}-{}: {}",
                diagnostic.v.level,
                src_path.display(),
                start,
                end,
                diagnostic.v.message,
            );
        }
    }

    let loader = loader.borrow();
    let file = File::create(&dest_path).expect("failed to create output file");
    let writer = BufWriter::new(file);
    pdf::export(&layouts, &loader, writer).expect("failed to export pdf");
}
