use std::env;
use std::ffi::OsStr;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use tiny_skia as sk;
use walkdir::WalkDir;

use typst::diag::Error;
use typst::eval::{Smart, StyleMap, Value};
use typst::frame::{Element, Frame};
use typst::geom::{Length, RgbaColor};
use typst::library::{PageNode, TextNode};
use typst::loading::FsLoader;
use typst::parse::Scanner;
use typst::source::SourceFile;
use typst::syntax::Span;
use typst::Context;

#[cfg(feature = "layout-cache")]
use {
    filedescriptor::{FileDescriptor, StdioDescriptor::*},
    std::fs::File,
    typst::layout::RootNode,
};

const TYP_DIR: &str = "./typ";
const REF_DIR: &str = "./ref";
const PNG_DIR: &str = "./png";
const PDF_DIR: &str = "./pdf";
const FONT_DIR: &str = "../fonts";

fn main() {
    env::set_current_dir(env::current_dir().unwrap().join("tests")).unwrap();

    let args = Args::new(env::args().skip(1));
    let mut filtered = Vec::new();

    // Since differents tests can affect each other through the layout cache, a
    // deterministic order is very important for reproducibility.
    for entry in WalkDir::new(".").sort_by_file_name() {
        let entry = entry.unwrap();
        if entry.depth() <= 1 {
            continue;
        }

        let src_path = entry.into_path();
        if src_path.extension() != Some(OsStr::new("typ")) {
            continue;
        }

        if args.matches(&src_path) {
            filtered.push(src_path);
        }
    }

    let len = filtered.len();
    if len == 1 {
        println!("Running test ...");
    } else if len > 1 {
        println!("Running {len} tests");
    }

    // Set page width to 120pt with 10pt margins, so that the inner page is
    // exactly 100pt wide. Page height is unbounded and font size is 10pt so
    // that it multiplies to nice round numbers.
    let mut styles = StyleMap::new();
    styles.set(PageNode::WIDTH, Smart::Custom(Length::pt(120.0)));
    styles.set(PageNode::HEIGHT, Smart::Auto);
    styles.set(PageNode::LEFT, Smart::Custom(Length::pt(10.0).into()));
    styles.set(PageNode::TOP, Smart::Custom(Length::pt(10.0).into()));
    styles.set(PageNode::RIGHT, Smart::Custom(Length::pt(10.0).into()));
    styles.set(PageNode::BOTTOM, Smart::Custom(Length::pt(10.0).into()));
    styles.set(TextNode::SIZE, Length::pt(10.0).into());

    // Hook up an assert function into the global scope.
    let mut std = typst::library::new();
    std.def_const("conifer", RgbaColor::new(0x9f, 0xEB, 0x52, 0xFF));
    std.def_const("forest", RgbaColor::new(0x43, 0xA1, 0x27, 0xFF));
    std.def_func("test", move |_, args| {
        let lhs = args.expect::<Value>("left-hand side")?;
        let rhs = args.expect::<Value>("right-hand side")?;
        if lhs != rhs {
            return Err(Error::boxed(
                args.span,
                format!("Assertion failed: {:?} != {:?}", lhs, rhs),
            ));
        }
        Ok(Value::None)
    });

    // Create loader and context.
    let loader = FsLoader::new().with_path(FONT_DIR).wrap();
    let mut ctx = Context::builder().std(std).styles(styles).build(loader);

    // Run all the tests.
    let mut ok = 0;
    for src_path in filtered {
        let path = src_path.strip_prefix(TYP_DIR).unwrap();
        let png_path = Path::new(PNG_DIR).join(path).with_extension("png");
        let ref_path = Path::new(REF_DIR).join(path).with_extension("png");
        let pdf_path =
            args.pdf.then(|| Path::new(PDF_DIR).join(path).with_extension("pdf"));

        ok += test(
            &mut ctx,
            &src_path,
            &png_path,
            &ref_path,
            pdf_path.as_deref(),
            args.debug,
        ) as usize;
    }

    if len > 1 {
        println!("{ok} / {len} tests passed.");
    }

    if ok < len {
        std::process::exit(1);
    }
}

struct Args {
    filter: Vec<String>,
    exact: bool,
    debug: bool,
    pdf: bool,
}

impl Args {
    fn new(args: impl Iterator<Item = String>) -> Self {
        let mut filter = Vec::new();
        let mut exact = false;
        let mut debug = false;
        let mut pdf = false;

        for arg in args {
            match arg.as_str() {
                // Ignore this, its for cargo.
                "--nocapture" => {}
                // Match only the exact filename.
                "--exact" => exact = true,
                // Generate PDFs.
                "--pdf" => pdf = true,
                // Debug print the layout trees.
                "--debug" | "-d" => debug = true,
                // Everything else is a file filter.
                _ => filter.push(arg),
            }
        }

        Self { filter, pdf, debug, exact }
    }

    fn matches(&self, path: &Path) -> bool {
        if self.exact {
            let name = path.file_name().unwrap().to_string_lossy();
            self.filter.iter().any(|v| v == &name)
        } else {
            let path = path.to_string_lossy();
            self.filter.is_empty() || self.filter.iter().any(|v| path.contains(v))
        }
    }
}

fn test(
    ctx: &mut Context,
    src_path: &Path,
    png_path: &Path,
    ref_path: &Path,
    pdf_path: Option<&Path>,
    debug: bool,
) -> bool {
    let name = src_path.strip_prefix(TYP_DIR).unwrap_or(src_path);
    println!("Testing {}", name.display());

    let src = fs::read_to_string(src_path).unwrap();

    let mut ok = true;
    let mut frames = vec![];
    let mut line = 0;
    let mut compare_ref = true;
    let mut compare_ever = false;
    let mut rng = LinearShift::new();

    let parts: Vec<_> = src.split("\n---").collect();
    for (i, &part) in parts.iter().enumerate() {
        let is_header = i == 0
            && parts.len() > 1
            && part
                .lines()
                .all(|s| s.starts_with("//") || s.chars().all(|c| c.is_whitespace()));

        if is_header {
            for line in part.lines() {
                if line.starts_with("// Ref: false") {
                    compare_ref = false;
                }
            }
        } else {
            let (part_ok, compare_here, part_frames) = test_part(
                ctx,
                src_path,
                part.into(),
                i,
                compare_ref,
                line,
                debug,
                &mut rng,
            );
            ok &= part_ok;
            compare_ever |= compare_here;
            frames.extend(part_frames);
        }

        line += part.lines().count() + 1;
    }

    if compare_ever {
        if let Some(pdf_path) = pdf_path {
            let pdf_data = typst::export::pdf(ctx, &frames);
            fs::create_dir_all(&pdf_path.parent().unwrap()).unwrap();
            fs::write(pdf_path, pdf_data).unwrap();
        }

        let canvas = render(ctx, &frames);
        fs::create_dir_all(&png_path.parent().unwrap()).unwrap();
        canvas.save_png(png_path).unwrap();

        if let Ok(ref_pixmap) = sk::Pixmap::load_png(ref_path) {
            if canvas != ref_pixmap {
                println!("  Does not match reference image. ❌");
                ok = false;
            }
        } else if !frames.is_empty() {
            println!("  Failed to open reference image. ❌");
            ok = false;
        }
    }

    if ok {
        if !debug {
            print!("\x1b[1A");
        }
        println!("Testing {} ✔", name.display());
    }

    ok
}

fn test_part(
    ctx: &mut Context,
    src_path: &Path,
    src: String,
    i: usize,
    compare_ref: bool,
    line: usize,
    debug: bool,
    rng: &mut LinearShift,
) -> (bool, bool, Vec<Arc<Frame>>) {
    let mut ok = true;

    let id = ctx.sources.provide(src_path, src);
    let source = ctx.sources.get(id);
    if debug {
        println!("Syntax: {:#?}", source.root())
    }

    let (local_compare_ref, mut ref_errors) = parse_metadata(&source);
    let compare_ref = local_compare_ref.unwrap_or(compare_ref);

    ok &= test_reparse(ctx.sources.get(id).src(), i, rng);

    let (frames, mut errors) = match ctx.evaluate(id) {
        Ok(module) => {
            let tree = module.into_root();
            if debug {
                println!("Layout: {tree:#?}");
            }

            let mut frames = tree.layout(ctx);

            #[cfg(feature = "layout-cache")]
            (ok &= test_incremental(ctx, i, &tree, &frames));

            if !compare_ref {
                frames.clear();
            }

            (frames, vec![])
        }
        Err(errors) => (vec![], *errors),
    };

    // TODO: Also handle errors from other files.
    errors.retain(|error| error.span.source == id);
    for error in &mut errors {
        error.trace.clear();
    }

    // The comparison never fails since all spans are from the same source file.
    ref_errors.sort_by(|a, b| a.span.partial_cmp(&b.span).unwrap());
    errors.sort_by(|a, b| a.span.partial_cmp(&b.span).unwrap());

    if errors != ref_errors {
        println!("  Subtest {i} does not match expected errors. ❌");
        ok = false;

        let source = ctx.sources.get(id);
        for error in errors.iter() {
            if error.span.source == id && !ref_errors.contains(error) {
                print!("    Not annotated | ");
                print_error(&source, line, error);
            }
        }

        for error in ref_errors.iter() {
            if !errors.contains(error) {
                print!("    Not emitted   | ");
                print_error(&source, line, error);
            }
        }
    }

    (ok, compare_ref, frames)
}

fn parse_metadata(source: &SourceFile) -> (Option<bool>, Vec<Error>) {
    let mut compare_ref = None;
    let mut errors = vec![];

    let lines: Vec<_> = source.src().lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("// Ref: false") {
            compare_ref = Some(false);
        }

        if line.starts_with("// Ref: true") {
            compare_ref = Some(true);
        }

        let rest = if let Some(rest) = line.strip_prefix("// Error: ") {
            rest
        } else {
            continue;
        };

        fn num(s: &mut Scanner) -> usize {
            s.eat_while(|c| c.is_numeric()).parse().unwrap()
        }

        let comments =
            lines[i ..].iter().take_while(|line| line.starts_with("//")).count();

        let pos = |s: &mut Scanner| -> usize {
            let first = num(s) - 1;
            let (delta, column) =
                if s.eat_if(':') { (first, num(s) - 1) } else { (0, first) };
            let line = (i + comments) + delta;
            source.line_column_to_byte(line, column).unwrap()
        };

        let mut s = Scanner::new(rest);
        let start = pos(&mut s);
        let end = if s.eat_if('-') { pos(&mut s) } else { start };
        let span = Span::new(source.id(), start, end);

        errors.push(Error::new(span, s.rest().trim()));
    }

    (compare_ref, errors)
}

fn print_error(source: &SourceFile, line: usize, error: &Error) {
    let start_line = 1 + line + source.byte_to_line(error.span.start).unwrap();
    let start_col = 1 + source.byte_to_column(error.span.start).unwrap();
    let end_line = 1 + line + source.byte_to_line(error.span.end).unwrap();
    let end_col = 1 + source.byte_to_column(error.span.end).unwrap();
    println!(
        "Error: {start_line}:{start_col}-{end_line}:{end_col}: {}",
        error.message,
    );
}

/// Pseudorandomly edit the source file and test whether a reparse produces the
/// same result as a clean parse.
///
/// The method will first inject 10 strings once every 400 source characters
/// and then select 5 leaf node boundries to inject an additional, randomly
/// chosen string from the injection list.
fn test_reparse(src: &str, i: usize, rng: &mut LinearShift) -> bool {
    let supplements = [
        "[",
        "]",
        "{",
        "}",
        "(",
        ")",
        "#rect()",
        "a word",
        ", a: 1",
        "10.0",
        ":",
        "if i == 0 {true}",
        "for",
        "* hello *",
        "//",
        "/*",
        "\\u{12e4}",
        "```typst",
        " ",
        "trees",
        "\\",
        "$ a $",
        "2.",
        "-",
        "5",
    ];

    let mut ok = true;

    let apply = |replace: std::ops::Range<usize>, with| {
        let mut incr_source = SourceFile::detached(src);
        if incr_source.root().len() != src.len() {
            println!(
                "    Subtest {i} tree length {} does not match string length {} ❌",
                incr_source.root().len(),
                src.len(),
            );
            return false;
        }

        incr_source.edit(replace.clone(), with);

        let edited_src = incr_source.src();
        let ref_source = SourceFile::detached(edited_src);
        let incr_root = incr_source.root();
        let ref_root = ref_source.root();
        let same = incr_root == ref_root;
        if !same {
            println!(
                "    Subtest {i} reparse differs from clean parse when inserting '{with}' at {}-{} ❌\n",
                replace.start, replace.end,
            );
            println!("    Expected reference tree:\n{ref_root:#?}\n");
            println!("    Found incremental tree:\n{incr_root:#?}");
            println!("Full source ({}):\n\"{edited_src:?}\"", edited_src.len());
        }

        same
    };

    let mut pick = |range: Range<usize>| {
        let ratio = rng.next();
        (range.start as f64 + ratio * (range.end - range.start) as f64).floor() as usize
    };

    let insertions = (src.len() as f64 / 400.0).ceil() as usize;
    for _ in 0 .. insertions {
        let supplement = supplements[pick(0 .. supplements.len())];
        let start = pick(0 .. src.len());
        let end = pick(start .. src.len());

        if !src.is_char_boundary(start) || !src.is_char_boundary(end) {
            continue;
        }

        ok &= apply(start .. end, supplement);
    }

    let red = SourceFile::detached(src).red();
    let leafs = red.as_ref().leafs();
    let leaf_start = leafs[pick(0 .. leafs.len())].span().start;
    let supplement = supplements[pick(0 .. supplements.len())];
    ok &= apply(leaf_start .. leaf_start, supplement);

    ok
}

#[cfg(feature = "layout-cache")]
fn test_incremental(
    ctx: &mut Context,
    i: usize,
    tree: &RootNode,
    frames: &[Arc<Frame>],
) -> bool {
    let mut ok = true;

    let reference = ctx.layout_cache.clone();
    for level in 0 .. reference.levels() {
        ctx.layout_cache = reference.clone();
        ctx.layout_cache.retain(|x| x == level);
        if ctx.layout_cache.is_empty() {
            continue;
        }

        ctx.layout_cache.turnaround();

        let cached = silenced(|| tree.layout(ctx));
        let total = reference.levels() - 1;
        let misses = ctx
            .layout_cache
            .entries()
            .filter(|e| e.level() == level && !e.hit() && e.age() == 2)
            .count();

        if misses > 0 {
            println!(
                "    Subtest {i} relayout had {misses} cache misses on level {level} of {total} ❌",
            );
            ok = false;
        }

        if cached != frames {
            println!(
                "    Subtest {i} relayout differs from clean pass on level {level} ❌",
            );
            ok = false;
        }
    }

    ctx.layout_cache = reference;
    ctx.layout_cache.turnaround();

    ok
}

/// Draw all frames into one image with padding in between.
fn render(ctx: &mut Context, frames: &[Arc<Frame>]) -> sk::Pixmap {
    let pixel_per_pt = 2.0;
    let pixmaps: Vec<_> = frames
        .iter()
        .map(|frame| {
            let limit = Length::cm(100.0);
            if frame.size.x > limit || frame.size.y > limit {
                panic!("overlarge frame: {:?}", frame.size);
            }
            typst::export::render(ctx, frame, pixel_per_pt)
        })
        .collect();

    let pad = (5.0 * pixel_per_pt).round() as u32;
    let pxw = 2 * pad + pixmaps.iter().map(sk::Pixmap::width).max().unwrap_or_default();
    let pxh = pad + pixmaps.iter().map(|pixmap| pixmap.height() + pad).sum::<u32>();

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();
    canvas.fill(sk::Color::BLACK);

    let [x, mut y] = [pad; 2];
    for (frame, mut pixmap) in frames.iter().zip(pixmaps) {
        let ts = sk::Transform::from_scale(pixel_per_pt, pixel_per_pt);
        render_links(&mut pixmap, ts, ctx, frame);

        canvas.draw_pixmap(
            x as i32,
            y as i32,
            pixmap.as_ref(),
            &sk::PixmapPaint::default(),
            sk::Transform::identity(),
            None,
        );

        y += pixmap.height() + pad;
    }

    canvas
}

/// Draw extra boxes for links so we can see whether they are there.
fn render_links(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    ctx: &Context,
    frame: &Frame,
) {
    for (pos, element) in &frame.elements {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *element {
            Element::Group(ref group) => {
                let ts = ts.pre_concat(group.transform.into());
                render_links(canvas, ts, ctx, &group.frame);
            }
            Element::Link(_, size) => {
                let w = size.x.to_pt() as f32;
                let h = size.y.to_pt() as f32;
                let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
                let mut paint = sk::Paint::default();
                paint.set_color_rgba8(40, 54, 99, 40);
                canvas.fill_rect(rect, &paint, ts, None);
            }
            _ => {}
        }
    }
}

/// Disable stdout and stderr during execution of `f`.
#[cfg(feature = "layout-cache")]
fn silenced<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let path = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let null = File::create(path).unwrap();
    let stderr = FileDescriptor::redirect_stdio(&null, Stderr).unwrap();
    let stdout = FileDescriptor::redirect_stdio(&null, Stdout).unwrap();
    let result = f();
    FileDescriptor::redirect_stdio(&stderr, Stderr).unwrap();
    FileDescriptor::redirect_stdio(&stdout, Stdout).unwrap();
    result
}

/// This is an Linear-feedback shift register using XOR as its shifting
/// function. It can be used as PRNG.
struct LinearShift(u64);

impl LinearShift {
    /// Initialize the shift register with a pre-set seed.
    pub fn new() -> Self {
        Self(0xACE5)
    }

    /// Return a pseudo-random number between `0.0` and `1.0`.
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 3;
        self.0 ^= self.0 << 14;
        self.0 ^= self.0 >> 28;
        self.0 ^= self.0 << 36;
        self.0 ^= self.0 >> 52;
        self.0 as f64 / u64::MAX as f64
    }
}
