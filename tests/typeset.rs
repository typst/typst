use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use fontdock::FsIndex;
use image::{GenericImageView, Rgba};
use tiny_skia::{
    Color, ColorU8, FillRule, FilterQuality, Paint, Pattern, Pixmap, Rect, SpreadMode,
    Transform,
};
use ttf_parser::OutlineBuilder;
use walkdir::WalkDir;

use typst::color;
use typst::diag::{Diag, DiagSet, Level, Pass};
use typst::env::{Env, FsIndexExt, ImageResource, ResourceLoader};
use typst::eval::{EvalContext, FuncArgs, FuncValue, Scope, Value};
use typst::exec::State;
use typst::export::pdf;
use typst::geom::{self, Length, Point, Sides, Size};
use typst::layout::{Element, Fill, Frame, Geometry, Image, Shape, Shaped};
use typst::library;
use typst::parse::{LineMap, Scanner};
use typst::pretty::pretty;
use typst::syntax::{Location, Pos};
use typst::typeset;

const TYP_DIR: &str = "./typ";
const REF_DIR: &str = "./ref";
const PNG_DIR: &str = "./png";
const PDF_DIR: &str = "./pdf";
const FONT_DIR: &str = "../fonts";

fn main() {
    env::set_current_dir(env::current_dir().unwrap().join("tests")).unwrap();

    let args = Args::new(env::args().skip(1));
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

        if args.matches(&src_path.to_string_lossy()) {
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

    let mut ok = true;
    for src_path in filtered {
        let path = src_path.strip_prefix(TYP_DIR).unwrap();
        let png_path = Path::new(PNG_DIR).join(path).with_extension("png");
        let ref_path = Path::new(REF_DIR).join(path).with_extension("png");
        let pdf_path =
            args.pdf.then(|| Path::new(PDF_DIR).join(path).with_extension("pdf"));

        ok &= test(
            &mut env,
            &src_path,
            &png_path,
            &ref_path,
            pdf_path.as_deref(),
        );
    }

    if !ok {
        std::process::exit(1);
    }
}

struct Args {
    filter: Vec<String>,
    pdf: bool,
    perfect: bool,
}

impl Args {
    fn new(args: impl Iterator<Item = String>) -> Self {
        let mut filter = Vec::new();
        let mut perfect = false;
        let mut pdf = false;

        for arg in args {
            match arg.as_str() {
                "--nocapture" => {}
                "--pdf" => pdf = true,
                "=" => perfect = true,
                _ => filter.push(arg),
            }
        }

        Self { filter, pdf, perfect }
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
    env: &mut Env,
    src_path: &Path,
    png_path: &Path,
    ref_path: &Path,
    pdf_path: Option<&Path>,
) -> bool {
    let name = src_path.strip_prefix(TYP_DIR).unwrap_or(src_path);
    println!("Testing {}", name.display());

    let src = fs::read_to_string(src_path).unwrap();

    let mut ok = true;
    let mut frames = vec![];
    let mut lines = 0;
    let mut compare_ref = true;
    let mut compare_ever = false;

    let parts: Vec<_> = src.split("---").collect();
    for (i, part) in parts.iter().enumerate() {
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
            let (part_ok, compare_here, part_frames) =
                test_part(env, part, i, compare_ref, lines);
            ok &= part_ok;
            compare_ever |= compare_here;
            frames.extend(part_frames);
        }

        lines += part.lines().count() as u32;
    }

    if compare_ever {
        if let Some(pdf_path) = pdf_path {
            let pdf_data = pdf::export(&env, &frames);
            fs::create_dir_all(&pdf_path.parent().unwrap()).unwrap();
            fs::write(pdf_path, pdf_data).unwrap();
        }

        let canvas = draw(&env, &frames, 2.0);
        fs::create_dir_all(&png_path.parent().unwrap()).unwrap();
        canvas.save_png(png_path).unwrap();

        if let Ok(ref_pixmap) = Pixmap::load_png(ref_path) {
            if canvas != ref_pixmap {
                println!("  Does not match reference image. ❌");
                ok = false;
            }
        } else {
            println!("  Failed to open reference image. ❌");
            ok = false;
        }
    }

    if ok {
        println!("\x1b[1ATesting {} ✔", name.display());
    }

    ok
}

fn test_part(
    env: &mut Env,
    src: &str,
    i: usize,
    compare_ref: bool,
    lines: u32,
) -> (bool, bool, Vec<Frame>) {
    let map = LineMap::new(src);
    let (local_compare_ref, ref_diags) = parse_metadata(src, &map);
    let compare_ref = local_compare_ref.unwrap_or(compare_ref);

    let mut scope = library::_new();

    let panics = Rc::new(RefCell::new(vec![]));
    register_helpers(&mut scope, Rc::clone(&panics));

    // We want to have "unbounded" pages, so we allow them to be infinitely
    // large and fit them to match their content.
    let mut state = State::default();
    state.page.size = Size::new(Length::pt(120.0), Length::raw(f64::INFINITY));
    state.page.margins = Sides::uniform(Some(Length::pt(10.0).into()));

    let Pass { output: mut frames, diags } = typeset(env, &src, &scope, state);
    if !compare_ref {
        frames.clear();
    }

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

    (ok, compare_ref, frames)
}

fn parse_metadata(src: &str, map: &LineMap) -> (Option<bool>, DiagSet) {
    let mut diags = DiagSet::new();
    let mut compare_ref = None;

    for (i, line) in src.lines().enumerate() {
        let line = line.trim();

        if line.starts_with("// Ref: false") {
            compare_ref = Some(false);
        }

        if line.starts_with("// Ref: true") {
            compare_ref = Some(true);
        }

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
        let start = pos(&mut s);
        let end = if s.eat_if('-') { pos(&mut s) } else { start };

        diags.insert(Diag::new(start .. end, level, s.rest().trim()));
    }

    (compare_ref, diags)
}

struct Panic {
    pos: Pos,
    lhs: Option<Value>,
    rhs: Option<Value>,
}

fn register_helpers(scope: &mut Scope, panics: Rc<RefCell<Vec<Panic>>>) {
    pub fn args(_: &mut EvalContext, args: &mut FuncArgs) -> Value {
        let repr = pretty(args);
        args.items.clear();
        Value::template("args", move |ctx| {
            let snapshot = ctx.state.clone();
            ctx.set_monospace();
            ctx.push_text(&repr);
            ctx.state = snapshot;
        })
    }

    let test = move |ctx: &mut EvalContext, args: &mut FuncArgs| -> Value {
        let lhs = args.require::<Value>(ctx, "left-hand side");
        let rhs = args.require::<Value>(ctx, "right-hand side");
        if lhs != rhs {
            panics.borrow_mut().push(Panic { pos: args.span.start, lhs, rhs });
            Value::Str(format!("(panic)"))
        } else {
            Value::None
        }
    };

    scope.def_const("error", Value::Error);
    scope.def_const("args", FuncValue::new(Some("args".into()), args));
    scope.def_const("test", FuncValue::new(Some("test".into()), test));
}

fn print_diag(diag: &Diag, map: &LineMap, lines: u32) {
    let mut start = map.location(diag.span.start).unwrap();
    let mut end = map.location(diag.span.end).unwrap();
    start.line += lines;
    end.line += lines;
    println!("{}: {}-{}: {}", diag.level, start, end, diag.message);
}

fn draw(env: &Env, frames: &[Frame], pixel_per_pt: f32) -> Pixmap {
    let pad = Length::pt(5.0);

    let height = pad + frames.iter().map(|l| l.size.height + pad).sum::<Length>();
    let width = 2.0 * pad
        + frames
            .iter()
            .map(|l| l.size.width)
            .max_by(|a, b| a.partial_cmp(&b).unwrap())
            .unwrap_or_default();

    let pixel_width = (pixel_per_pt * width.to_pt() as f32) as u32;
    let pixel_height = (pixel_per_pt * height.to_pt() as f32) as u32;
    if pixel_width > 4000 || pixel_height > 4000 {
        panic!(
            "overlarge image: {} by {} ({} x {})",
            pixel_width, pixel_height, width, height,
        );
    }

    let mut canvas = Pixmap::new(pixel_width, pixel_height).unwrap();
    let ts = Transform::from_scale(pixel_per_pt, pixel_per_pt);
    canvas.fill(Color::BLACK);

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
            ts,
            None,
        );

        for &(pos, ref element) in &frame.elements {
            let pos = origin + pos;
            match element {
                Element::Text(shaped) => {
                    draw_text(&mut canvas, env, ts, pos, shaped);
                }
                Element::Image(image) => {
                    draw_image(&mut canvas, env, ts, pos, image);
                }
                Element::Geometry(geom) => {
                    draw_geometry(&mut canvas, ts, pos, geom);
                }
            }
        }

        origin.y += frame.size.height + pad;
    }

    canvas
}

fn draw_text(canvas: &mut Pixmap, env: &Env, ts: Transform, pos: Point, shaped: &Shaped) {
    let face = env.fonts.face(shaped.face).get();

    for (&glyph, &offset) in shaped.glyphs.iter().zip(&shaped.offsets) {
        let units_per_em = face.units_per_em().unwrap_or(1000);

        let x = (pos.x + offset).to_pt() as f32;
        let y = pos.y.to_pt() as f32;
        let scale = (shaped.font_size / units_per_em as f64).to_pt() as f32;

        // Try drawing SVG if present.
        if let Some(tree) = face
            .glyph_svg_image(glyph)
            .and_then(|data| std::str::from_utf8(data).ok())
            .map(|svg| {
                let viewbox = format!("viewBox=\"0 0 {0} {0}\" xmlns", units_per_em);
                svg.replace("xmlns", &viewbox)
            })
            .and_then(|s| usvg::Tree::from_str(&s, &usvg::Options::default()).ok())
        {
            for child in tree.root().children() {
                if let usvg::NodeKind::Path(node) = &*child.borrow() {
                    let path = convert_usvg_path(&node.data);
                    let transform = convert_usvg_transform(node.transform);
                    let ts = transform
                        .post_concat(Transform::from_row(scale, 0.0, 0.0, scale, x, y))
                        .post_concat(ts);

                    if let Some(fill) = &node.fill {
                        let (paint, fill_rule) = convert_usvg_fill(fill);
                        canvas.fill_path(&path, &paint, fill_rule, ts, None);
                    }
                }
            }

            continue;
        }

        // Otherwise, draw normal outline.
        let mut builder = WrappedPathBuilder(tiny_skia::PathBuilder::new());
        if face.outline_glyph(glyph, &mut builder).is_some() {
            let path = builder.0.finish().unwrap();
            let ts = Transform::from_row(scale, 0.0, 0.0, -scale, x, y).post_concat(ts);
            let mut paint = convert_typst_fill(shaped.color);
            paint.anti_alias = true;
            canvas.fill_path(&path, &paint, FillRule::default(), ts, None);
        }
    }
}

fn draw_geometry(canvas: &mut Pixmap, ts: Transform, pos: Point, element: &Geometry) {
    let x = pos.x.to_pt() as f32;
    let y = pos.y.to_pt() as f32;
    let ts = Transform::from_translate(x, y).post_concat(ts);

    let paint = convert_typst_fill(element.fill);
    let rule = FillRule::default();

    match element.shape {
        Shape::Rect(Size { width, height }) => {
            let w = width.to_pt() as f32;
            let h = height.to_pt() as f32;
            let rect = Rect::from_xywh(0.0, 0.0, w, h).unwrap();
            canvas.fill_rect(rect, &paint, ts, None);
        }
        Shape::Ellipse(size) => {
            let path = convert_typst_path(&geom::ellipse_path(size));
            canvas.fill_path(&path, &paint, rule, ts, None);
        }
        Shape::Path(ref path) => {
            let path = convert_typst_path(path);
            canvas.fill_path(&path, &paint, rule, ts, None);
        }
    };
}

fn draw_image(
    canvas: &mut Pixmap,
    env: &Env,
    ts: Transform,
    pos: Point,
    element: &Image,
) {
    let img = &env.resources.loaded::<ImageResource>(element.res);

    let mut pixmap = Pixmap::new(img.buf.width(), img.buf.height()).unwrap();
    for ((_, _, src), dest) in img.buf.pixels().zip(pixmap.pixels_mut()) {
        let Rgba([r, g, b, a]) = src;
        *dest = ColorU8::from_rgba(r, g, b, a).premultiply();
    }

    let x = pos.x.to_pt() as f32;
    let y = pos.y.to_pt() as f32;
    let view_width = element.size.width.to_pt() as f32;
    let view_height = element.size.height.to_pt() as f32;
    let scale_x = view_width as f32 / pixmap.width() as f32;
    let scale_y = view_height as f32 / pixmap.height() as f32;

    let mut paint = Paint::default();
    paint.shader = Pattern::new(
        pixmap.as_ref(),
        SpreadMode::Pad,
        FilterQuality::Bilinear,
        1.0,
        Transform::from_row(scale_x, 0.0, 0.0, scale_y, x, y),
    );

    let rect = Rect::from_xywh(x, y, view_width, view_height).unwrap();
    canvas.fill_rect(rect, &paint, ts, None);
}

fn convert_typst_fill(fill: Fill) -> Paint<'static> {
    let mut paint = Paint::default();
    match fill {
        Fill::Color(c) => match c {
            color::Color::Rgba(c) => paint.set_color_rgba8(c.r, c.g, c.b, c.a),
        },
        Fill::Image(_) => todo!(),
    }
    paint
}

fn convert_typst_path(path: &geom::Path) -> tiny_skia::Path {
    let f = |length: Length| length.to_pt() as f32;
    let mut builder = tiny_skia::PathBuilder::new();
    for elem in &path.0 {
        match elem {
            geom::PathElement::MoveTo(p) => builder.move_to(f(p.x), f(p.y)),
            geom::PathElement::LineTo(p) => builder.line_to(f(p.x), f(p.y)),
            geom::PathElement::CubicTo(p1, p2, p3) => {
                builder.cubic_to(f(p1.x), f(p1.y), f(p2.x), f(p2.y), f(p3.x), f(p3.y))
            }
            geom::PathElement::ClosePath => builder.close(),
        };
    }
    builder.finish().unwrap()
}

fn convert_usvg_fill(fill: &usvg::Fill) -> (Paint<'static>, FillRule) {
    let mut paint = Paint::default();
    paint.anti_alias = true;

    match fill.paint {
        usvg::Paint::Color(color) => paint.set_color_rgba8(
            color.red,
            color.green,
            color.blue,
            fill.opacity.to_u8(),
        ),
        usvg::Paint::Link(_) => {}
    }

    let rule = match fill.rule {
        usvg::FillRule::NonZero => FillRule::Winding,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    };

    (paint, rule)
}

fn convert_usvg_path(path: &usvg::PathData) -> tiny_skia::Path {
    let f = |v: f64| v as f32;
    let mut builder = tiny_skia::PathBuilder::new();
    for seg in path.iter() {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => builder.move_to(f(x), f(y)),
            usvg::PathSegment::LineTo { x, y } => {
                builder.line_to(f(x), f(y));
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                builder.cubic_to(f(x1), f(y1), f(x2), f(y2), f(x), f(y))
            }
            usvg::PathSegment::ClosePath => builder.close(),
        }
    }
    builder.finish().unwrap()
}

fn convert_usvg_transform(transform: usvg::Transform) -> Transform {
    let g = |v: f64| v as f32;
    let usvg::Transform { a, b, c, d, e, f } = transform;
    Transform::from_row(g(a), g(b), g(c), g(d), g(e), g(f))
}

struct WrappedPathBuilder(tiny_skia::PathBuilder);

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
