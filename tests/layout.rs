use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{File, create_dir_all, read_dir, read_to_string};
use std::io::{BufWriter, Write};
use std::panic;
use std::process::Command;

use futures_executor::block_on;

use typstc::Typesetter;
use typstc::layout::{MultiLayout, Serialize};
use typstc::size::{Size, Size2D};
use typstc::style::PageStyle;
use typstc::toddle::query::FileSystemFontProvider;
use typstc::export::pdf::PdfExporter;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let opts = Options::parse();

    create_dir_all("tests/cache/serial")?;
    create_dir_all("tests/cache/render")?;
    create_dir_all("tests/cache/pdf")?;

    let tests: Vec<_> = read_dir("tests/layouts/")?.collect();
    let mut filtered = Vec::new();

    for entry in tests {
        let path = entry?.path();
        if path.extension() != Some(OsStr::new("typ")) {
            continue;
        }

        let name = path
            .file_stem().ok_or("expected file stem")?
            .to_string_lossy()
            .to_string();

        if opts.matches(&name) {
            let src = read_to_string(&path)?;
            filtered.push((name, src));
        }
    }

    let len = filtered.len();
    println!();
    println!("Running {} test{}", len, if len > 1 { "s" } else { "" });

    for (name, src) in filtered {
        panic::catch_unwind(|| {
            if let Err(e) = test(&name, &src) {
                println!("error: {}", e);
            }
        }).ok();
    }

    println!();

    Ok(())
}

/// Create a _PDF_ with a name from the source code.
fn test(name: &str, src: &str) -> Result<()> {
    println!("Testing: {}.", name);

    let mut typesetter = Typesetter::new();
    typesetter.set_page_style(PageStyle {
        dimensions: Size2D::with_all(Size::pt(250.0)),
        .. PageStyle::default()
    });

    let provider = FileSystemFontProvider::from_index("../fonts/index.json")?;
    let font_paths = provider.paths();
    typesetter.add_font_provider(provider);

    let layouts = match compile(&typesetter, src) {
        Some(layouts) => layouts,
        None => return Ok(()),
    };

    // Compute the font's paths.
    let mut fonts = HashMap::new();
    let loader = typesetter.loader().borrow();
    for layout in &layouts {
        for index in layout.find_used_fonts() {
            fonts.entry(index).or_insert_with(|| {
                let p = loader.get_provider_and_index(index.id).1;
                &font_paths[p][index.variant]
            });
        }
    }
    drop(loader);

    // Write the serialized layout file.
    let path = format!("tests/cache/serial/{}", name);
    let mut file = BufWriter::new(File::create(path)?);

    // Write the font mapping into the serialization file.
    writeln!(file, "{}", fonts.len())?;
    for (index, path) in fonts.iter() {
        writeln!(file, "{} {} {}", index.id, index.variant, path)?;
    }
    layouts.serialize(&mut file)?;

    // Render the layout into a PNG.
    Command::new("python")
        .arg("tests/render.py")
        .arg(name)
        .spawn()
        .expect("failed to run python renderer");

    // Write the PDF file.
    let path = format!("tests/cache/pdf/{}.pdf", name);
    let file = BufWriter::new(File::create(path)?);
    let exporter = PdfExporter::new();
    exporter.export(&layouts, typesetter.loader(), file)?;

    Ok(())
}

/// Compile the source code with the typesetter.
fn compile(typesetter: &Typesetter, src: &str) -> Option<MultiLayout> {
    #[cfg(not(debug_assertions))] {
        use std::time::Instant;

        // Warmup.
        let warmup_start = Instant::now();
        let is_ok = block_on(typesetter.typeset(&src)).is_ok();
        let warmup_end = Instant::now();

        // Only continue if the typesetting was successful.
        if is_ok {
            let start = Instant::now();
            let tree = typesetter.parse(&src).unwrap();
            let mid = Instant::now();
            block_on(typesetter.layout(&tree)).unwrap();
            let end = Instant::now();

            println!(" - cold start:  {:?}", warmup_end - warmup_start);
            println!(" - warmed up:   {:?}", end - start);
            println!("   - parsing:   {:?}", mid - start);
            println!("   - layouting: {:?}", end - mid);
            println!();
        }
    };

    match block_on(typesetter.typeset(&src)) {
        Ok(layouts) => Some(layouts),
        Err(err) => {
            println!(" - compilation failed: {}", err);
            #[cfg(not(debug_assertions))]
            println!();
            None
        }
    }
}

/// Command line options.
struct Options {
    filter: Vec<String>,
    perfect: bool,
}

impl Options {
    /// Parse the options from the environment arguments.
    fn parse() -> Options {
        let mut perfect = false;
        let mut filter = Vec::new();

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--nocapture" => {},
                "=" => perfect = true,
                _ => filter.push(arg),
            }
        }

        Options { filter, perfect }
    }

    /// Whether a given test should be executed.
    fn matches(&self, name: &str) -> bool {
        match self.perfect {
            true => self.filter.iter().any(|p| name == p),
            false => self.filter.is_empty()
                || self.filter.iter().any(|p| name.contains(p))
        }
    }
}
