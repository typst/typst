use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::process::Command;
use std::time::Instant;

use regex::{Regex, Captures};

use typst::export::pdf::PdfExporter;
use typst::layout::LayoutAction;
use typst::toddle::query::FileSystemFontProvider;
use typst::size::{Size, Size2D, SizeBox};
use typst::style::PageStyle;
use typst::Typesetter;

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

    for entry in fs::read_dir("tests/layouts/").unwrap() {
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

            test(name, &src);
        }
    }
}

/// Create a _PDF_ with a name from the source code.
fn test(name: &str, src: &str) {
    println!("Testing: {}", name);

    let (src, size) = preprocess(src);

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider.clone());

    if let Some(dimensions) = size {
        typesetter.set_page_style(PageStyle {
            dimensions,
            margins: SizeBox::zero()
        });
    }

    let start = Instant::now();

    // Layout into box layout.
    let tree = typesetter.parse(&src).unwrap();
    let layouts = typesetter.layout(&tree).unwrap();

    let end = Instant::now();
    let duration = end - start;
    println!(" => {:?}", duration);
    println!();

    // Write the serialed layout file.
    let path = format!("{}/serialized/{}.lay", CACHE_DIR, name);
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

fn preprocess<'a>(src: &'a str) -> (String, Option<Size2D>) {
    let include_regex = Regex::new(r"\{include:((.|\.|\-)*)\}").unwrap();
    let lorem_regex = Regex::new(r"\{lorem:(\d*)\}").unwrap();
    let size_regex = Regex::new(r"\{(size:(([\d\w]*)\*([\d\w]*)))\}").unwrap();

    let mut size = None;

    let mut preprocessed = size_regex.replace_all(&src, |cap: &Captures| {
        let width_str = cap.get(3).unwrap().as_str();
        let height_str = cap.get(4).unwrap().as_str();

        let width = width_str.parse::<Size>().unwrap();
        let height = height_str.parse::<Size>().unwrap();

        size = Some(Size2D::new(width, height));

        "".to_string()
    }).to_string();

    let mut changed = true;
    while changed {
        changed = false;
        preprocessed = include_regex.replace_all(&preprocessed, |cap: &Captures| {
            changed = true;
            let filename = cap.get(1).unwrap().as_str();

            let path = format!("tests/layouts/{}", filename);
            let mut file = File::open(path).unwrap();
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            buf
        }).to_string();
    }

    preprocessed= lorem_regex.replace_all(&preprocessed, |cap: &Captures| {
        let num_str = cap.get(1).unwrap().as_str();
        let num_words = num_str.parse::<usize>().unwrap();

        generate_lorem(num_words)
    }).to_string();

    (preprocessed, size)
}

fn generate_lorem(num_words: usize) -> String {
    const LOREM: [&str; 69] = [
        "Lorem", "ipsum", "dolor", "sit", "amet,", "consectetur", "adipiscing", "elit.", "Etiam",
        "suscipit", "porta", "pretium.", "Donec", "eu", "lorem", "hendrerit,", "scelerisque",
        "lectus", "at,", "consequat", "ligula.", "Nulla", "elementum", "massa", "et", "viverra",
        "consectetur.", "Donec", "blandit", "metus", "ut", "ipsum", "commodo", "congue.", "Nullam",
        "auctor,", "mi", "vel", "tristique", "venenatis,", "nisl", "nunc", "tristique", "diam,",
        "aliquam", "pellentesque", "lorem", "massa", "vel", "neque.", "Sed", "malesuada", "ante",
        "nisi,", "sit", "amet", "auctor", "risus", "fermentum", "in.", "Sed", "blandit", "mollis",
        "mi,", "non", "tristique", "nisi", "fringilla", "at."
    ];

    let mut buf = String::new();
    for i in 0 .. num_words {
        buf.push_str(LOREM[i % LOREM.len()]);
        buf.push(' ');
    }
    buf
}
