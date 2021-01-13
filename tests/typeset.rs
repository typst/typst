use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsSource};
use image::{GenericImageView, Rgba};
use tiny_skia::{
    Canvas, Color, ColorU8, FillRule, FilterQuality, Paint, PathBuilder, Pattern, Pixmap,
    Rect, SpreadMode, Transform,
};
use ttf_parser::OutlineBuilder;

use typst::diag::{Diag, Feedback, Level, Pass};
use typst::env::{Env, ImageResource, ResourceLoader, SharedEnv};
use typst::eval::{Args, EvalContext, State, Value, ValueFunc};
use typst::export::pdf;
use typst::font::FontLoader;
use typst::geom::{Length, Point, Sides, Size};
use typst::layout::{Element, Frame, Image};
use typst::parse::{LineMap, Scanner};
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

    for entry in fs::read_dir(TYP_DIR).unwrap() {
        let src_path = entry.unwrap().path();
        if src_path.extension() != Some(OsStr::new("typ")) {
            continue;
        }

        let name = src_path.file_stem().unwrap().to_string_lossy().to_string();
        if filter.matches(&name) {
            filtered.push((name, src_path));
        }
    }

    let len = filtered.len();
    if len == 0 {
        return;
    } else if len == 1 {
        println!("Running test ...");
    } else {
        println!("Running {} tests", len);
    }

    fs::create_dir_all(PNG_DIR).unwrap();
    fs::create_dir_all(PDF_DIR).unwrap();

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (files, descriptors) = index.into_vecs();
    let env = Rc::new(RefCell::new(Env {
        fonts: FontLoader::new(Box::new(FsSource::new(files)), descriptors),
        resources: ResourceLoader::new(),
    }));

    let mut ok = true;

    for (name, src_path) in filtered {
        let png_path = Path::new(PNG_DIR).join(&name).with_extension("png");
        let pdf_path = Path::new(PDF_DIR).join(&name).with_extension("pdf");
        let ref_path = Path::new(REF_DIR).join(&name).with_extension("png");
        ok &= test(
            &name,
            &src_path,
            &png_path,
            &pdf_path,
            Some(&ref_path),
            &env,
        );
    }

    let playground = Path::new("playground.typ");
    if playground.exists() {
        test(
            "playground",
            playground,
            Path::new("playground.png"),
            Path::new("playground.pdf"),
            None,
            &env,
        );
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
    name: &str,
    src_path: &Path,
    png_path: &Path,
    pdf_path: &Path,
    ref_path: Option<&Path>,
    env: &SharedEnv,
) -> bool {
    println!("Testing {}.", name);

    let src = fs::read_to_string(src_path).unwrap();

    let mut ok = true;
    let mut frames = vec![];

    for (i, part) in src.split("---").enumerate() {
        let (part_ok, part_frames) = test_part(i, part, env);
        ok &= part_ok;
        frames.extend(part_frames);
    }

    let env = env.borrow();
    if !frames.is_empty() {
        let canvas = draw(&frames, &env, 2.0);
        canvas.pixmap.save_png(png_path).unwrap();

        let pdf_data = pdf::export(&frames, &env);
        fs::write(pdf_path, pdf_data).unwrap();

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
        println!("\x1b[1ATesting {}. ✔", name);
    }

    ok
}

fn test_part(i: usize, src: &str, env: &SharedEnv) -> (bool, Vec<Frame>) {
    let (src, compare_ref, map, ref_diags) = parse_metadata(&src, i);

    let mut state = State::default();
    state.page.size = Size::uniform(Length::pt(120.0));
    state.page.margins = Sides::uniform(Some(Length::pt(10.0).into()));

    pub fn dump(_: &mut EvalContext, args: &mut Args) -> Value {
        let (array, dict) = args.drain();
        Value::Array(vec![Value::Array(array), Value::Dict(dict)])
    }

    Rc::make_mut(&mut state.scope).set("dump", ValueFunc::new("dump", dump));

    let Pass {
        output: mut frames,
        feedback: Feedback { mut diags, .. },
    } = typeset(&src, Rc::clone(env), state);

    if !compare_ref {
        frames.clear();
    }

    diags.sort_by_key(|d| d.span);

    let mut ok = true;
    if diags != ref_diags {
        println!("  Subtest {} does not match expected diagnostics. ❌", i);
        ok = false;

        for diag in &diags {
            if ref_diags.binary_search(diag).is_err() {
                print!("    Unexpected | ");
                print_diag(diag, &map);
            }
        }

        for diag in &ref_diags {
            if diags.binary_search(diag).is_err() {
                print!("    Missing    | ");
                print_diag(diag, &map);
            }
        }
    }

    (ok, frames)
}

fn parse_metadata(src: &str, i: usize) -> (&str, bool, LineMap, SpanVec<Diag>) {
    let mut diags = vec![];
    let mut compare_ref = true;

    let mut s = Scanner::new(src);
    for k in 0 .. {
        // Allow a newline directly after "---" (that is, if i > 0 and k == 0).
        if !(i > 0 && k == 0) && !s.rest().starts_with("//") {
            break;
        }

        let line = s.eat_until(typst::parse::is_newline);
        s.eat_merging_crlf();

        compare_ref &= !line.starts_with("// ref: false");

        let (level, rest) = if let Some(rest) = line.strip_prefix("// warning: ") {
            (Level::Warning, rest)
        } else if let Some(rest) = line.strip_prefix("// error: ") {
            (Level::Error, rest)
        } else {
            continue;
        };

        diags.push((level, rest));
    }

    let src = s.rest();
    let map = LineMap::new(src);

    let mut diags: Vec<_> = diags
        .into_iter()
        .map(|(level, rest)| {
            fn pos(s: &mut Scanner, map: &LineMap) -> Pos {
                let (line, _, column) = (num(s), s.eat_assert(':'), num(s));
                map.pos(Location { line, column }).unwrap()
            }

            fn num(s: &mut Scanner) -> u32 {
                s.eat_while(|c| c.is_numeric()).parse().unwrap()
            }

            let mut s = Scanner::new(rest);
            let (start, _, end) =
                (pos(&mut s, &map), s.eat_assert('-'), pos(&mut s, &map));
            Diag::new(level, s.rest().trim()).with_span(start .. end)
        })
        .collect();

    diags.sort_by_key(|d| d.span);

    (src, compare_ref, map, diags)
}

fn print_diag(diag: &Spanned<Diag>, map: &LineMap) {
    let start = map.location(diag.span.start).unwrap();
    let end = map.location(diag.span.end).unwrap();
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
