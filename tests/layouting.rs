use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::process::Command;
use std::time::Instant;

use typst::export::pdf::PdfExporter;
use typst::layout::LayoutAction;
use typst::toddle::query::FileSystemFontProvider;
use typst::Typesetter;

const CACHE_DIR: &str = "test-cache";

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

    for entry in fs::read_dir("tests/layouts/").unwrap() {
        let path = entry.unwrap().path();

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

            test(name, &src);
        }
    }
}

/// Create a _PDF_ with a name from the source code.
fn test(name: &str, src: &str) {
    print!("Testing: {}", name);

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider.clone());

    let start = Instant::now();

    // Layout into box layout.
    let tree = typesetter.parse(src).unwrap();
    let layout = typesetter.layout(&tree).unwrap();

    let end = Instant::now();
    let duration = end - start;
    println!(" [{:?}]", duration);

    // Write the serialed layout file.
    let path = format!("{}/serialized/{}.box", CACHE_DIR, name);
    let mut file = File::create(path).unwrap();

    // Find all used fonts and their filenames.
    let mut map = Vec::new();
    let mut loader = typesetter.loader().borrow_mut();
    let single = &layout.layouts[0];
    for action in &single.actions {
        if let LayoutAction::SetFont(index, _) = action {
            if map.iter().find(|(i, _)| i == index).is_none() {
                let (_, provider_index) = loader.get_provider_and_index(*index);
                let filename = provider.get_path(provider_index).to_str().unwrap();
                map.push((*index, filename));
            }
        }
    }
    drop(loader);

    // Write the font mapping into the serialization file.
    writeln!(file, "{}", map.len()).unwrap();
    for (index, path) in map {
        writeln!(file, "{} {}", index, path).unwrap();
    }
    single.serialize(&mut file).unwrap();

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
    exporter.export(&layout, typesetter.loader(), file).unwrap();
}
