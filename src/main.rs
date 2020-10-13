use std::cell::RefCell;
use std::fs::{read_to_string, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsSource};

use typst::diag::{Feedback, Pass};
use typst::eval::State;
use typst::export::pdf;
use typst::font::FontLoader;
use typst::parse::LineMap;
use typst::typeset;

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

    let (files, descriptors) = index.into_vecs();
    let loader = Rc::new(RefCell::new(FontLoader::new(
        Box::new(FsSource::new(files)),
        descriptors,
    )));

    let state = State::default();
    let Pass {
        output: layouts,
        feedback: Feedback { mut diags, .. },
    } = typeset(&src, state, Rc::clone(&loader));

    if !diags.is_empty() {
        diags.sort();

        let map = LineMap::new(&src);
        for diag in diags {
            let span = diag.span;
            let start = map.location(span.start);
            let end = map.location(span.end);
            println!(
                "  {}: {}:{}-{}: {}",
                diag.v.level,
                src_path.display(),
                start,
                end,
                diag.v.message,
            );
        }
    }

    let loader = loader.borrow();
    let file = File::create(&dest_path).expect("failed to create output file");
    let writer = BufWriter::new(file);
    pdf::export(&layouts, &loader, writer).expect("failed to export pdf");
}
