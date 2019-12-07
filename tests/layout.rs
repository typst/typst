use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::process::Command;

use typstc::export::pdf::PdfExporter;
use typstc::layout::{LayoutAction, Serialize};
use typstc::size::{Size, Size2D, SizeBox};
use typstc::style::PageStyle;
use typstc::toddle::query::FileSystemFontProvider;
use typstc::Typesetter;

const CACHE_DIR: &str = "tests/cache";

fn main() {
    let mut perfect_match = false;
    let mut filter = Vec::new();

    for arg in std::env::args().skip(1) {
        if arg.as_str() == "--nocapture" {
            continue;
        } else if arg.as_str() == "=" {
            perfect_match = true;
        } else {
            filter.push(arg);
        }
    }

    fs::create_dir_all(format!("{}/serialized", CACHE_DIR)).unwrap();
    fs::create_dir_all(format!("{}/rendered", CACHE_DIR)).unwrap();
    fs::create_dir_all(format!("{}/pdf", CACHE_DIR)).unwrap();

    let mut failed = 0;

    for entry in fs::read_dir("tests/layouting/").unwrap() {
        let path = entry.unwrap().path();

        if path.extension() != Some(std::ffi::OsStr::new("typ")) {
            continue;
        }

        let name = path.file_stem().unwrap().to_str().unwrap();

        let matches = if perfect_match {
            filter.iter().any(|pattern| name == pattern)
        } else {
            filter.is_empty() || filter.iter().any(|pattern| name.contains(pattern))
        };

        if matches {
            let mut file = File::open(&path).unwrap();
            let mut src = String::new();
            file.read_to_string(&mut src).unwrap();

            if std::panic::catch_unwind(|| test(name, &src)).is_err() {
                failed += 1;
                println!();
            }
        }
    }

    if failed > 0 {
        println!("{} tests failed.", failed);
        println!();
        std::process::exit(-1);
    }

    println!();
}

/// Create a _PDF_ with a name from the source code.
fn test(name: &str, src: &str) {
    println!("Testing: {}.", name);

    let mut typesetter = Typesetter::new();

    typesetter.set_page_style(PageStyle {
        dimensions: Size2D::with_all(Size::pt(250.0)),
        margins: SizeBox::with_all(Size::pt(10.0)),
    });

    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider.clone());

    #[cfg(not(debug_assertions))] {
        use std::time::Instant;

        // Warmup.
        let warmup_start = Instant::now();
        let is_ok = typesetter.typeset(&src).is_ok();
        let warmup_end = Instant::now();

        if is_ok {
            let start = Instant::now();
            let tree = typesetter.parse(&src).unwrap();
            let mid = Instant::now();
            typesetter.layout(&tree).unwrap();
            let end = Instant::now();

            println!(" - cold start:  {:?}", warmup_end - warmup_start);
            println!(" - warmed up:   {:?}", end - start);
            println!("   - parsing:   {:?}", mid - start);
            println!("   - layouting: {:?}", end - mid);
            println!();
        }
    };

    let layouts = match typesetter.typeset(&src) {
        Ok(layouts) => layouts,
        Err(err) => {
            println!(" - compilation failed: {}", err);
            #[cfg(not(debug_assertions))]
            println!();
            return;
        }
    };

    // Write the serialed layout file.
    let path = format!("{}/serialized/{}.tld", CACHE_DIR, name);
    let mut file = File::create(path).unwrap();

    // Find all used fonts and their filenames.
    let mut map = Vec::new();
    let mut loader = typesetter.loader().borrow_mut();
    for layout in &layouts {
        for action in &layout.actions {
            if let LayoutAction::SetFont(index, _) = action {
                if map.iter().find(|(i, _)| i == index).is_none() {
                    let (_, provider_index) = loader.get_provider_and_index(*index);
                    let filename = provider.get_path(provider_index).to_str().unwrap();
                    map.push((*index, filename));
                }
            }
        }
    }
    drop(loader);

    // Write the font mapping into the serialization file.
    writeln!(file, "{}", map.len()).unwrap();
    for (index, path) in map {
        writeln!(file, "{} {}", index, path).unwrap();
    }

    layouts.serialize(&mut file).unwrap();

    // Render the layout into a PNG.
    Command::new("python")
        .arg("tests/render.py")
        .arg(name)
        .spawn()
        .expect("failed to run python-based renderer");

    // Write the PDF file.
    let path = format!("{}/pdf/{}.pdf", CACHE_DIR, name);
    let file = BufWriter::new(File::create(path).unwrap());
    let exporter = PdfExporter::new();
    exporter.export(&layouts, typesetter.loader(), file).unwrap();
}
