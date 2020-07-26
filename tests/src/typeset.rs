use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{File, create_dir_all, read_dir, read_to_string};
use std::io::BufWriter;
use std::panic;
use std::process::Command;
use std::time::{Instant, Duration};

use serde::Serialize;
use futures_executor::block_on;

use typstc::{Typesetter, DebugErrorProvider};
use typstc::layout::MultiLayout;
use typstc::size::{Size, Size2D, ValueBox};
use typstc::style::{PageStyle, PaperClass};
use typstc::export::pdf;
use toddle::query::FontIndex;
use toddle::query::fs::EagerFsProvider;


type DynResult<T> = Result<T, Box<dyn Error>>;

fn main() -> DynResult<()> {
    let opts = Options::parse();

    create_dir_all("tests/cache")?;

    let tests: Vec<_> = read_dir("tests/")?.collect();
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

    Ok(())
}

/// Create a _PDF_ and render with a name from the source code.
fn test(name: &str, src: &str) -> DynResult<()> {
    println!("Testing: {}.", name);

    let (fs, entries) = EagerFsProvider::from_index("../fonts", "index.json")?;
    let files = fs.files().to_vec();
    let provider = DebugErrorProvider::new(fs);
    let mut typesetter = Typesetter::new((Box::new(provider), entries));

    typesetter.set_page_style(PageStyle {
        class: PaperClass::Custom,
        dimensions: Size2D::with_all(Size::pt(250.0)),
        margins: ValueBox::with_all(None),
    });

    let layouts = compile(&typesetter, src);

    // Write the PDF file.
    let path = format!("tests/cache/{}.pdf", name);
    let file = BufWriter::new(File::create(path)?);
    pdf::export(&layouts, typesetter.loader(), file)?;

    // Compute the font's paths.
    let mut fonts = HashMap::new();
    for layout in &layouts {
        for index in layout.find_used_fonts() {
            fonts.entry(index)
                .or_insert_with(|| files[index.id][index.variant].as_str());
        }
    }

    #[derive(Serialize)]
    struct Document<'a> {
        fonts: Vec<(FontIndex, &'a str)>,
        layouts: MultiLayout,
    }

    let document = Document { fonts: fonts.into_iter().collect(), layouts};

    // Serialize the document into JSON.
    let path = format!("tests/cache/{}.serde.json", name);
    let file = BufWriter::new(File::create(&path)?);
    serde_json::to_writer(file, &document)?;

    // Render the layout into a PNG.
    Command::new("python")
        .arg("tests/src/render.py")
        .arg(name)
        .spawn()
        .expect("failed to run python renderer")
        .wait()
        .expect("command did not run");

    std::fs::remove_file(path)?;

    Ok(())
}

/// Compile the source code with the typesetter.
fn compile(typesetter: &Typesetter, src: &str) -> MultiLayout {
    if cfg!(debug_assertions) {
        let typeset = block_on(typesetter.typeset(src));
        let problems = typeset.feedback.problems;

        if !problems.is_empty() {
            for problem in problems {
                println!("  {:?} {:?}: {}",
                    problem.v.severity,
                    problem.span,
                    problem.v.message
                );
            }
        }

        typeset.output
    } else {
        fn measure<T>(f: impl FnOnce() -> T) -> (T, Duration) {
            let start = Instant::now();
            let output = f();
            let duration = Instant::now() - start;
            (output, duration)
        };

        let (_, cold) = measure(|| block_on(typesetter.typeset(src)));
        let (model, parse) = measure(|| typesetter.parse(src).output);
        let (layouts, layout) = measure(|| block_on(typesetter.layout(&model)).output);

        println!(" - cold start:  {:?}", cold);
        println!(" - warmed up:   {:?}", parse + layout);
        println!("   - parsing:   {:?}", parse);
        println!("   - layouting: {:?}", layout);

        layouts
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
