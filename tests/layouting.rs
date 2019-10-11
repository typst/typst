use std::fs::{self, File};
use std::io::{Write, Read, BufWriter};
use std::process::Command;

use typst::Typesetter;
use typst::layout::LayoutAction;
use typst::toddle::query::FileSystemFontProvider;
use typst::export::pdf::PdfExporter;

const CACHE_DIR: &str = "test-cache";


fn main() {
    let mut filter = Vec::new();
    for arg in std::env::args().skip(1) {
        if arg.as_str() != "--nocapture" {
            filter.push(arg);
        }
    }

    if !filter.is_empty() {
        println!("Using filter: {:?}", filter);
    }

    fs::create_dir_all(format!("{}/serialized", CACHE_DIR)).unwrap();
    fs::create_dir_all(format!("{}/rendered", CACHE_DIR)).unwrap();
    fs::create_dir_all(format!("{}/pdf", CACHE_DIR)).unwrap();

    for entry in fs::read_dir("tests/layouts/").unwrap() {
        let path = entry.unwrap().path();

        let name = path
            .file_stem().unwrap()
            .to_str().unwrap();

        if filter.is_empty() || filter.iter().any(|pattern| name.contains(pattern)) {
            let mut file = File::open(&path).unwrap();
            let mut src = String::new();
            file.read_to_string(&mut src).unwrap();

            test(name, &src);
        }
    }
}

/// Create a _PDF_ with a name from the source code.
fn test(name: &str, src: &str) {
    println!("Testing: {}", name);

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider.clone());

    // Layout into box layout.
    let tree = typesetter.parse(src).unwrap();
    let layout = typesetter.layout(&tree).unwrap();

    // Write the serialed layout file.
    let path = format!("{}/serialized/{}.box", CACHE_DIR, name);
    let mut file = File::create(path).unwrap();

    // Find all used fonts and their filenames.
    let mut map = Vec::new();
    let mut loader = typesetter.loader().borrow_mut();
    for action in &layout.actions {
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
    layout.serialize(&mut file).unwrap();

    // Render the layout into a PNG.
    Command::new("python")
        .arg("tests/render.py")
        .arg(name)
        .spawn()
        .expect("failed to run python-based renderer");

    // Write the PDF file.
    let path = format!("{}/pdf/{}.pdf", CACHE_DIR, name);
    let file = BufWriter::new(File::create(path).unwrap());
    let document = layout.into_doc();
    let exporter = PdfExporter::new();
    exporter.export(&document, typesetter.loader(), file).unwrap();
}
