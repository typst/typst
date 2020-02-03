use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{File, create_dir_all, read_dir, read_to_string};
use std::io::{BufWriter, Write};
use std::panic;
use std::process::Command;

use futures_executor::block_on;

use typstc::{Typesetter, DynErrorProvider};
use typstc::layout::{MultiLayout, Serialize};
use typstc::size::{Size, Size2D, ValueBox};
use typstc::style::{PageStyle, PaperClass};
use typstc::export::pdf;
use typstc::toddle::query::fs::EagerFsProvider;


type DynResult<T> = Result<T, Box<dyn Error>>;

fn main() -> DynResult<()> {
    let opts = Options::parse();

    create_dir_all("tests/cache/serial")?;
    create_dir_all("tests/cache/render")?;
    create_dir_all("tests/cache/pdf")?;

    let tests: Vec<_> = read_dir("tests/layouter/")?.collect();
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
                println!("error: {:?}", e);
            }
        }).ok();
    }

    println!();

    Ok(())
}

/// Create a _PDF_ and render with a name from the source code.
fn test(name: &str, src: &str) -> DynResult<()> {
    println!("Testing: {}.", name);

    let (fs, entries) = EagerFsProvider::from_index("../fonts", "index.json")?;
    let paths = fs.paths();
    let provider = DynErrorProvider::new(fs);
    let mut typesetter = Typesetter::new((Box::new(provider), entries));

    typesetter.set_page_style(PageStyle {
        class: PaperClass::Custom,
        dimensions: Size2D::with_all(Size::pt(250.0)),
        margins: ValueBox::with_all(None),
    });

    let layouts = compile(&typesetter, src);

    // Compute the font's paths.
    let mut fonts = HashMap::new();
    let loader = typesetter.loader().borrow();
    for layout in &layouts {
        for index in layout.find_used_fonts() {
            fonts.entry(index)
                .or_insert_with(|| &paths[index.id][index.variant]);
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
        .arg("tests/src/render.py")
        .arg(name)
        .spawn()
        .expect("failed to run python renderer");

    // Write the PDF file.
    let path = format!("tests/cache/pdf/{}.pdf", name);
    let file = BufWriter::new(File::create(path)?);
    pdf::export(&layouts, typesetter.loader(), file)?;

    Ok(())
}

/// Compile the source code with the typesetter.
fn compile(typesetter: &Typesetter, src: &str) -> MultiLayout {
    #![allow(unused_variables)]
    use std::time::Instant;

    // Warmup.
    #[cfg(not(debug_assertions))]
    let warmup = {
        let warmup_start = Instant::now();
        block_on(typesetter.typeset(&src));
        Instant::now() - warmup_start
    };

    let start = Instant::now();
    let parsed = typesetter.parse(&src);
    let parse = Instant::now() - start;

    if !parsed.errors.is_empty() {
        println!("parse errors: {:#?}", parsed.errors);
    }

    let start_layout = Instant::now();
    let layouted = block_on(typesetter.layout(&parsed.output));
    let layout = Instant::now() - start_layout;
    let total = Instant::now() - start;

    if !layouted.errors.is_empty() {
        println!("layout errors: {:#?}", layouted.errors);
    }

    #[cfg(not(debug_assertions))] {
        println!(" - cold start:  {:?}", warmup);
        println!(" - warmed up:   {:?}", total);
        println!("   - parsing:   {:?}", parse);
        println!("   - layouting: {:?}", layout);
        println!();
    }

    layouted.output
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
