use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use fontdock::fs::FsIndex;
use image::{GenericImageView, Rgba};
use tiny_skia::{
    Canvas, Color, ColorU8, FillRule, FilterQuality, Paint, PathBuilder, Pattern, Pixmap,
    Rect, SpreadMode, Transform,
};
use ttf_parser::OutlineBuilder;
use walkdir::WalkDir;

use typst::diag::{Diag, Feedback, Level, Pass};
use typst::env::{Env, ImageResource, ResourceLoader};
use typst::eval::{Args, EvalContext, Scope, State, Value, ValueFunc};
use typst::export::pdf;
use typst::font::FsIndexExt;
use typst::geom::{Length, Point, Sides, Size, Spec};
use typst::layout::{Element, Expansion, Frame, Image};
use typst::library;
use typst::parse::{LineMap, Scanner};
use typst::pretty::{Pretty, Printer};
use typst::shaping::Shaped;
use typst::syntax::{Location, Pos, SpanVec, Spanned, WithSpan};
use typst::typeset;

const TYP_DIR: &str = "typ";
const REF_DIR: &str = "ref";
const PNG_DIR: &str = "png";
const PDF_DIR: &str = "pdf";
const FONT_DIR: &str = "../fonts";

fn main() {
    env::set_current_dir(env::current_dir().unwrap().join("tests")).unwrap();

    let filter = TestFilter::new(env::args().skip(1));
    let mut filtered = Vec::new();

    for entry in WalkDir::new(".").into_iter() {
        let entry = entry.unwrap();
        if entry.depth() <= 1 {
            continue;
        }

        let src_path = entry.into_path();
        if src_path.extension() != Some(OsStr::new("typ")) {
            continue;
        }

        if filter.matches(&src_path.to_string_lossy()) {
            filtered.push(src_path);
        }
    }

    let len = filtered.len();
    if len == 1 {
        println!("Running test ...");
    } else if len > 1 {
        println!("Running {} tests", len);
    }

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let mut env = Env {
        fonts: index.into_dynamic_loader(),
        resources: ResourceLoader::new(),
    };

    let playground = Path::new("playground.typ");
    if playground.exists() && filtered.is_empty() {
        test(
            playground,
            Path::new("playground.png"),
            Path::new("playground.pdf"),
            None,
            &mut env,
        );
    }

    let mut ok = true;
    for src_path in filtered {
        let category = src_path.parent().unwrap().parent().unwrap();
        let name = src_path.file_stem().unwrap();
        let png_path = category.join(PNG_DIR).join(name).with_extension("png");
        let pdf_path = category.join(PDF_DIR).join(name).with_extension("pdf");
        let ref_path = category.join(REF_DIR).join(name).with_extension("png");
        ok &= test(&src_path, &png_path, &pdf_path, Some(&ref_path), &mut env);
    }

    if !ok {
        std::process::exit(1);
    }
}

struct TestFilter {
    filter: Vec<String>,
    perfect: bool,
}

impl TestFilter {
    fn new(args: impl Iterator<Item = String>) -> Self {
        let mut filter = Vec::new();
        let mut perfect = false;

        for arg in args {
            match arg.as_str() {
                "--nocapture" => {}
                "=" => perfect = true,
                _ => filter.push(arg),
            }
        }

        Self { filter, perfect }
    }

    fn matches(&self, name: &str) -> bool {
        if self.perfect {
            self.filter.iter().any(|p| name == p)
        } else {
            self.filter.is_empty() || self.filter.iter().any(|p| name.contains(p))
        }
    }
}

fn test(
    src_path: &Path,
    png_path: &Path,
    pdf_path: &Path,
    ref_path: Option<&Path>,
    env: &mut Env,
) -> bool {
    let name = src_path.strip_prefix(TYP_DIR).unwrap_or(src_path);
    println!("Testing {}", name.display());

    let src = fs::read_to_string(src_path).unwrap();

    let mut ok = true;
    let mut frames = vec![];

    let mut lines = 0;
    for (i, part) in src.split("---").enumerate() {
        let (part_ok, part_frames) = test_part(part, i, lines, env);
        ok &= part_ok;
        frames.extend(part_frames);
        lines += part.lines().count() as u32;
    }

    if !frames.is_empty() {
        let pdf_data = pdf::export(&frames, &env);
        fs::create_dir_all(&pdf_path.parent().unwrap()).unwrap();
        fs::write(pdf_path, pdf_data).unwrap();

        let canvas = draw(&frames, &env, 2.0);
        fs::create_dir_all(&png_path.parent().unwrap()).unwrap();
        canvas.pixmap.save_png(png_path).unwrap();

        if let Some(ref_path) = ref_path {
            if let Ok(ref_pixmap) = Pixmap::load_png(ref_path) {
                if canvas.pixmap != ref_pixmap {
                    println!("  Does not match reference image. ❌");
                    ok = false;
                }
            } else {
                println!("  Failed to open reference image. ❌");
                ok = false;
            }
        }
    }

    if ok {
        println!("\x1b[1ATesting {} ✔", name.display());
    }

    ok
}

fn test_part(src: &str, i: usize, lines: u32, env: &mut Env) -> (bool, Vec<Frame>) {
    let map = LineMap::new(src);
    let (compare_ref, ref_diags) = parse_metadata(src, &map);

    let mut scope = library::new();

    let panics = Rc::new(RefCell::new(vec![]));
    register_helpers(&mut scope, Rc::clone(&panics));

    // We want to have "unbounded" pages, so we allow them to be infinitely
    // large and fit them to match their content.
    let mut state = State::default();
    state.page.size = Size::new(Length::pt(120.0), Length::raw(f64::INFINITY));
    state.page.expand = Spec::new(Expansion::Fill, Expansion::Fit);
    state.page.margins = Sides::uniform(Some(Length::pt(10.0).into()));

    let Pass {
        output: mut frames,
        feedback: Feedback { mut diags, .. },
    } = typeset(&src, env, &scope, state);

    if !compare_ref {
        frames.clear();
    }

    diags.sort_by_key(|d| d.span);

    let mut ok = true;

    for panic in &*panics.borrow() {
        let line = map.location(panic.pos).unwrap().line;
        println!("  Assertion failed in line {} ❌", lines + line);
        if let (Some(lhs), Some(rhs)) = (&panic.lhs, &panic.rhs) {
            println!("    Left:  {:?}", lhs);
            println!("    Right: {:?}", rhs);
        } else {
            println!("    Missing argument.");
        }
        ok = false;
    }

    if diags != ref_diags {
        println!("  Subtest {} does not match expected diagnostics. ❌", i);
        ok = false;

        for diag in &diags {
            if !ref_diags.contains(diag) {
                print!("    Not annotated | ");
                print_diag(diag, &map, lines);
            }
        }

        for diag in &ref_diags {
            if !diags.contains(diag) {
                print!("    Not emitted   | ");
                print_diag(diag, &map, lines);
            }
        }
    }

    (ok, frames)
}

fn parse_metadata(src: &str, map: &LineMap) -> (bool, SpanVec<Diag>) {
    let mut diags = vec![];
    let mut compare_ref = true;

    for (i, line) in src.lines().enumerate() {
        compare_ref &= !line.starts_with("// Ref: false");

        let (level, rest) = if let Some(rest) = line.strip_prefix("// Warning: ") {
            (Level::Warning, rest)
        } else if let Some(rest) = line.strip_prefix("// Error: ") {
            (Level::Error, rest)
        } else {
            continue;
        };

        fn num(s: &mut Scanner) -> u32 {
            s.eat_while(|c| c.is_numeric()).parse().unwrap()
        }

        let pos = |s: &mut Scanner| -> Pos {
            let first = num(s);
            let (delta, column) =
                if s.eat_if(':') { (first, num(s)) } else { (1, first) };
            let line = i as u32 + 1 + delta;
            map.pos(Location::new(line, column)).unwrap()
        };

        let mut s = Scanner::new(rest);
        let (start, _, end) = (pos(&mut s), s.eat_assert('-'), pos(&mut s));

        diags.push(Diag::new(level, s.rest().trim()).with_span(start .. end));
    }

    diags.sort_by_key(|d| d.span);

    (compare_ref, diags)
}

struct Panic {
    pos: Pos,
    lhs: Option<Value>,
    rhs: Option<Value>,
}

fn register_helpers(scope: &mut Scope, panics: Rc<RefCell<Vec<Panic>>>) {
    pub fn f(_: &mut EvalContext, args: &mut Args) -> Value {
        let (array, dict) = args.drain();
        let iter = array
            .into_iter()
            .map(|v| (None, v))
            .chain(dict.into_iter().map(|(k, v)| (Some(k), v)));

        let mut p = Printer::new();
        p.push_str("f(");
        p.join(iter, ", ", |(key, value), p| {
            if let Some(key) = key {
                p.push_str(&key);
                p.push_str(": ");
            }
            value.pretty(p);
        });
        p.push_str(")");

        Value::Str(p.finish())
    }

    let test = move |ctx: &mut EvalContext, args: &mut Args| -> Value {
        let lhs = args.require::<Value>(ctx, "left-hand side");
        let rhs = args.require::<Value>(ctx, "right-hand side");
        if lhs != rhs {
            panics.borrow_mut().push(Panic { pos: args.span.start, lhs, rhs });
            Value::Str(format!("(panic)"))
        } else {
            Value::None
        }
    };

    scope.def_const("f", ValueFunc::new("f", f));
    scope.def_const("test", ValueFunc::new("test", test));
}

fn print_diag(diag: &Spanned<Diag>, map: &LineMap, lines: u32) {
    let mut start = map.location(diag.span.start).unwrap();
    let mut end = map.location(diag.span.end).unwrap();
    start.line += lines;
    end.line += lines;
    println!("{}: {}-{}: {}", diag.v.level, start, end, diag.v.message);
}

fn draw(frames: &[Frame], env: &Env, pixel_per_pt: f32) -> Canvas {
    let pad = Length::pt(5.0);

    let height = pad + frames.iter().map(|l| l.size.height + pad).sum::<Length>();
    let width = 2.0 * pad
        + frames
            .iter()
            .map(|l| l.size.width)
            .max_by(|a, b| a.partial_cmp(&b).unwrap())
            .unwrap();

    let pixel_width = (pixel_per_pt * width.to_pt() as f32) as u32;
    let pixel_height = (pixel_per_pt * height.to_pt() as f32) as u32;
    if pixel_width > 4000 || pixel_height > 4000 {
        panic!("overlarge image: {} by {}", pixel_width, pixel_height);
    }

    let mut canvas = Canvas::new(pixel_width, pixel_height).unwrap();
    canvas.scale(pixel_per_pt, pixel_per_pt);
    canvas.pixmap.fill(Color::BLACK);

    let mut origin = Point::new(pad, pad);
    for frame in frames {
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);

        canvas.fill_rect(
            Rect::from_xywh(
                origin.x.to_pt() as f32,
                origin.y.to_pt() as f32,
                frame.size.width.to_pt() as f32,
                frame.size.height.to_pt() as f32,
            )
            .unwrap(),
            &paint,
        );

        for &(pos, ref element) in &frame.elements {
            let pos = origin + pos;
            match element {
                Element::Text(shaped) => {
                    draw_text(&mut canvas, pos, env, shaped);
                }
                Element::Image(image) => {
                    draw_image(&mut canvas, pos, env, image);
                }
            }
        }

        origin.y += frame.size.height + pad;
    }

    canvas
}

fn draw_text(canvas: &mut Canvas, pos: Point, env: &Env, shaped: &Shaped) {
    let face = env.fonts.face(shaped.face).get();

    for (&glyph, &offset) in shaped.glyphs.iter().zip(&shaped.offsets) {
        let units_per_em = face.units_per_em().unwrap_or(1000);

        let x = (pos.x + offset).to_pt() as f32;
        let y = (pos.y + shaped.font_size).to_pt() as f32;
        let scale = (shaped.font_size / units_per_em as f64).to_pt() as f32;

        let mut builder = WrappedPathBuilder(PathBuilder::new());
        face.outline_glyph(glyph, &mut builder);

        if let Some(path) = builder.0.finish() {
            let placed = path
                .transform(&Transform::from_row(scale, 0.0, 0.0, -scale, x, y).unwrap())
                .unwrap();

            let mut paint = Paint::default();
            paint.anti_alias = true;

            canvas.fill_path(&placed, &paint, FillRule::default());
        }
    }
}

fn draw_image(canvas: &mut Canvas, pos: Point, env: &Env, element: &Image) {
    let img = &env.resources.loaded::<ImageResource>(element.res);

    let mut pixmap = Pixmap::new(img.buf.width(), img.buf.height()).unwrap();
    for ((_, _, src), dest) in img.buf.pixels().zip(pixmap.pixels_mut()) {
        let Rgba([r, g, b, a]) = src;
        *dest = ColorU8::from_rgba(r, g, b, a).premultiply();
    }

    let view_width = element.size.width.to_pt() as f32;
    let view_height = element.size.height.to_pt() as f32;

    let x = pos.x.to_pt() as f32;
    let y = pos.y.to_pt() as f32;
    let scale_x = view_width as f32 / pixmap.width() as f32;
    let scale_y = view_height as f32 / pixmap.height() as f32;

    let mut paint = Paint::default();
    paint.shader = Pattern::new(
        &pixmap,
        SpreadMode::Pad,
        FilterQuality::Bilinear,
        1.0,
        Transform::from_row(scale_x, 0.0, 0.0, scale_y, x, y).unwrap(),
    );

    canvas.fill_rect(
        Rect::from_xywh(x, y, view_width, view_height).unwrap(),
        &paint,
    );
}

struct WrappedPathBuilder(PathBuilder);

impl OutlineBuilder for WrappedPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.0.close();
    }
}
