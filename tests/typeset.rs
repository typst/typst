use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use image::{GenericImageView, Rgba};
use tiny_skia as sk;
use ttf_parser::{GlyphId, OutlineBuilder};
use walkdir::WalkDir;

use typst::color::{Color, RgbaColor};
use typst::diag::Error;
use typst::eval::{State, Value};
use typst::font::Face;
use typst::geom::{self, Length, PathElement, Point, Sides, Size};
use typst::image::Image;
#[cfg(feature = "layout-cache")]
use typst::layout::LayoutTree;
use typst::layout::{layout, Element, Frame, Geometry, Paint, Text};
use typst::loading::FsLoader;
use typst::parse::Scanner;
use typst::source::SourceFile;
use typst::syntax::{Pos, Span};
use typst::Context;

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

    // We want to have "unbounded" pages, so we allow them to be infinitely
    // large and fit them to match their content.
    let mut state = State::default();
    state.page_mut().size = Size::new(Length::pt(120.0), Length::inf());
    state.page_mut().margins = Sides::splat(Some(Length::pt(10.0).into()));
    state.font_mut().size = Length::pt(10.0);

    // Hook up an assert function into the global scope.
    let mut std = typst::library::new();
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
    let mut ctx = Context::builder().std(std).state(state).build(loader);

    // Run all the tests.
    let mut ok = true;
    for src_path in filtered {
        let path = src_path.strip_prefix(TYP_DIR).unwrap();
        let png_path = Path::new(PNG_DIR).join(path).with_extension("png");
        let ref_path = Path::new(REF_DIR).join(path).with_extension("png");
        let pdf_path =
            args.pdf.then(|| Path::new(PDF_DIR).join(path).with_extension("pdf"));

        ok &= test(
            &mut ctx,
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
    ctx: &mut Context,
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
    let mut line = 0;
    let mut compare_ref = true;
    let mut compare_ever = false;

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
            let (part_ok, compare_here, part_frames) =
                test_part(ctx, src_path, part.into(), i, compare_ref, line);
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

        let canvas = draw(ctx, &frames, 2.0);
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
        println!("\x1b[1ATesting {} ✔", name.display());
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
) -> (bool, bool, Vec<Rc<Frame>>) {
    let id = ctx.sources.provide(src_path, src);
    let source = ctx.sources.get(id);

    let (local_compare_ref, mut ref_errors) = parse_metadata(&source);
    let compare_ref = local_compare_ref.unwrap_or(compare_ref);

    let mut ok = true;
    let (frames, mut errors) = match ctx.execute(id) {
        Ok(tree) => {
            let mut frames = layout(ctx, &tree);

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
        println!("  Subtest {} does not match expected errors. ❌", i);
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

#[cfg(feature = "layout-cache")]
fn test_incremental(
    ctx: &mut Context,
    i: usize,
    tree: &LayoutTree,
    frames: &[Rc<Frame>],
) -> bool {
    let mut ok = true;

    let reference = ctx.layouts.clone();
    for level in 0 .. reference.levels() {
        ctx.layouts = reference.clone();
        ctx.layouts.retain(|x| x == level);
        if ctx.layouts.is_empty() {
            continue;
        }

        ctx.layouts.turnaround();

        let cached = layout(ctx, tree);
        let misses = ctx
            .layouts
            .entries()
            .filter(|e| e.level() == level && !e.hit() && e.age() == 2)
            .count();

        if misses > 0 {
            println!(
                "    Subtest {} relayout had {} cache misses on level {} of {} ❌",
                i,
                misses,
                level,
                reference.levels() - 1,
            );
            ok = false;
        }

        if cached != frames {
            println!("    Subtest {} relayout differs from clean pass ❌", i);
            ok = false;
        }
    }

    ctx.layouts = reference;
    ctx.layouts.turnaround();

    ok
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

        let pos = |s: &mut Scanner| -> Pos {
            let first = num(s) - 1;
            let (delta, column) =
                if s.eat_if(':') { (first, num(s) - 1) } else { (0, first) };
            let line = (i + comments) + delta;
            source.line_column_to_byte(line, column).unwrap().into()
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
    let start_line = 1 + line + source.byte_to_line(error.span.start.to_usize()).unwrap();
    let start_col = 1 + source.byte_to_column(error.span.start.to_usize()).unwrap();
    let end_line = 1 + line + source.byte_to_line(error.span.end.to_usize()).unwrap();
    let end_col = 1 + source.byte_to_column(error.span.end.to_usize()).unwrap();
    println!(
        "Error: {}:{}-{}:{}: {}",
        start_line, start_col, end_line, end_col, error.message
    );
}

fn draw(ctx: &Context, frames: &[Rc<Frame>], dpp: f32) -> sk::Pixmap {
    let pad = Length::pt(5.0);
    let width = 2.0 * pad + frames.iter().map(|l| l.size.w).max().unwrap_or_default();
    let height = pad + frames.iter().map(|l| l.size.h + pad).sum::<Length>();

    let pixel_width = (dpp * width.to_pt() as f32) as u32;
    let pixel_height = (dpp * height.to_pt() as f32) as u32;
    if pixel_width > 4000 || pixel_height > 4000 {
        panic!(
            "overlarge image: {} by {} ({:?} x {:?})",
            pixel_width, pixel_height, width, height,
        );
    }

    let mut canvas = sk::Pixmap::new(pixel_width, pixel_height).unwrap();
    let ts = sk::Transform::from_scale(dpp, dpp);
    canvas.fill(sk::Color::BLACK);

    let mut origin = Point::splat(pad);
    for frame in frames {
        let mut paint = sk::Paint::default();
        paint.set_color(sk::Color::WHITE);
        canvas.fill_rect(
            sk::Rect::from_xywh(
                origin.x.to_pt() as f32,
                origin.y.to_pt() as f32,
                frame.size.w.to_pt() as f32,
                frame.size.h.to_pt() as f32,
            )
            .unwrap(),
            &paint,
            ts,
            None,
        );

        for (pos, element) in frame.elements() {
            let global = origin + pos;
            let x = global.x.to_pt() as f32;
            let y = global.y.to_pt() as f32;
            let ts = ts.pre_translate(x, y);
            match *element {
                Element::Text(ref text) => {
                    draw_text(&mut canvas, ts, ctx.fonts.get(text.face_id), text);
                }
                Element::Geometry(ref geometry, paint) => {
                    draw_geometry(&mut canvas, ts, geometry, paint);
                }
                Element::Image(id, size) => {
                    draw_image(&mut canvas, ts, ctx.images.get(id), size);
                }
                Element::Link(_, s) => {
                    let outline = Geometry::Rect(s);
                    let paint = Paint::Color(Color::Rgba(RgbaColor::new(40, 54, 99, 40)));
                    draw_geometry(&mut canvas, ts, &outline, paint);
                }
            }
        }

        origin.y += frame.size.h + pad;
    }

    canvas
}

fn draw_text(canvas: &mut sk::Pixmap, ts: sk::Transform, face: &Face, text: &Text) {
    let ttf = face.ttf();
    let size = text.size.to_pt() as f32;
    let units_per_em = ttf.units_per_em() as f32;
    let pixels_per_em = text.size.to_pt() as f32 * ts.sy;
    let scale = size / units_per_em;

    let mut x = 0.0;
    for glyph in &text.glyphs {
        let glyph_id = GlyphId(glyph.id);
        let offset = x + glyph.x_offset.to_length(text.size).to_pt() as f32;
        let ts = ts.pre_translate(offset, 0.0);

        if let Some(tree) = ttf
            .glyph_svg_image(glyph_id)
            .and_then(|data| std::str::from_utf8(data).ok())
            .map(|svg| {
                let viewbox = format!("viewBox=\"0 0 {0} {0}\" xmlns", units_per_em);
                svg.replace("xmlns", &viewbox)
            })
            .and_then(|s| usvg::Tree::from_str(&s, &usvg::Options::default()).ok())
        {
            for child in tree.root().children() {
                if let usvg::NodeKind::Path(node) = &*child.borrow() {
                    // SVG is already Y-down, no flipping required.
                    let ts = convert_usvg_transform(node.transform)
                        .post_scale(scale, scale)
                        .post_concat(ts);

                    if let Some(fill) = &node.fill {
                        let path = convert_usvg_path(&node.data);
                        let (paint, fill_rule) = convert_usvg_fill(fill);
                        canvas.fill_path(&path, &paint, fill_rule, ts, None);
                    }
                }
            }
        } else if let Some(raster) =
            ttf.glyph_raster_image(glyph_id, pixels_per_em as u16)
        {
            // TODO: Vertical alignment isn't quite right for Apple Color Emoji,
            // and maybe also for Noto Color Emoji. And: Is the size calculation
            // correct?
            let img = Image::parse(&raster.data).unwrap();
            let h = text.size;
            let w = (img.width() as f64 / img.height() as f64) * h;
            let dx = (raster.x as f32) / (img.width() as f32) * size;
            let dy = (raster.y as f32) / (img.height() as f32) * size;
            let ts = ts.pre_translate(dx, -size - dy);
            draw_image(canvas, ts, &img, Size::new(w, h));
        } else {
            // Otherwise, draw normal outline.
            let mut builder = WrappedPathBuilder(sk::PathBuilder::new());
            if ttf.outline_glyph(glyph_id, &mut builder).is_some() {
                // Flip vertically because font designed coordinate system is Y-up.
                let ts = ts.pre_scale(scale, -scale);
                let path = builder.0.finish().unwrap();
                let mut paint = convert_typst_paint(text.fill);
                paint.anti_alias = true;
                canvas.fill_path(&path, &paint, sk::FillRule::default(), ts, None);
            }
        }

        x += glyph.x_advance.to_length(text.size).to_pt() as f32;
    }
}

fn draw_geometry(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    geometry: &Geometry,
    paint: Paint,
) {
    let paint = convert_typst_paint(paint);
    let rule = sk::FillRule::default();

    match *geometry {
        Geometry::Rect(Size { w: width, h: height }) => {
            let w = width.to_pt() as f32;
            let h = height.to_pt() as f32;
            let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
            canvas.fill_rect(rect, &paint, ts, None);
        }
        Geometry::Ellipse(size) => {
            let path = convert_typst_path(&geom::Path::ellipse(size));
            canvas.fill_path(&path, &paint, rule, ts, None);
        }
        Geometry::Line(target, thickness) => {
            let path = {
                let mut builder = sk::PathBuilder::new();
                builder.line_to(target.x.to_pt() as f32, target.y.to_pt() as f32);
                builder.finish().unwrap()
            };

            let mut stroke = sk::Stroke::default();
            stroke.width = thickness.to_pt() as f32;
            canvas.stroke_path(&path, &paint, &stroke, ts, None);
        }
        Geometry::Path(ref path) => {
            let path = convert_typst_path(path);
            canvas.fill_path(&path, &paint, rule, ts, None);
        }
    };
}

fn draw_image(canvas: &mut sk::Pixmap, ts: sk::Transform, img: &Image, size: Size) {
    let mut pixmap = sk::Pixmap::new(img.buf.width(), img.buf.height()).unwrap();
    for ((_, _, src), dest) in img.buf.pixels().zip(pixmap.pixels_mut()) {
        let Rgba([r, g, b, a]) = src;
        *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
    }

    let view_width = size.w.to_pt() as f32;
    let view_height = size.h.to_pt() as f32;
    let scale_x = view_width as f32 / pixmap.width() as f32;
    let scale_y = view_height as f32 / pixmap.height() as f32;

    let mut paint = sk::Paint::default();
    paint.shader = sk::Pattern::new(
        pixmap.as_ref(),
        sk::SpreadMode::Pad,
        sk::FilterQuality::Bilinear,
        1.0,
        sk::Transform::from_scale(scale_x, scale_y),
    );

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height).unwrap();
    canvas.fill_rect(rect, &paint, ts, None);
}

fn convert_typst_paint(paint: Paint) -> sk::Paint<'static> {
    let Paint::Color(Color::Rgba(c)) = paint;
    let mut paint = sk::Paint::default();
    paint.set_color_rgba8(c.r, c.g, c.b, c.a);
    paint
}

fn convert_typst_path(path: &geom::Path) -> sk::Path {
    let mut builder = sk::PathBuilder::new();
    let f = |v: Length| v.to_pt() as f32;
    for elem in &path.0 {
        match elem {
            PathElement::MoveTo(p) => {
                builder.move_to(f(p.x), f(p.y));
            }
            PathElement::LineTo(p) => {
                builder.line_to(f(p.x), f(p.y));
            }
            PathElement::CubicTo(p1, p2, p3) => {
                builder.cubic_to(f(p1.x), f(p1.y), f(p2.x), f(p2.y), f(p3.x), f(p3.y));
            }
            PathElement::ClosePath => {
                builder.close();
            }
        };
    }
    builder.finish().unwrap()
}

fn convert_usvg_transform(transform: usvg::Transform) -> sk::Transform {
    let g = |v: f64| v as f32;
    let usvg::Transform { a, b, c, d, e, f } = transform;
    sk::Transform::from_row(g(a), g(b), g(c), g(d), g(e), g(f))
}

fn convert_usvg_fill(fill: &usvg::Fill) -> (sk::Paint<'static>, sk::FillRule) {
    let mut paint = sk::Paint::default();
    paint.anti_alias = true;

    match fill.paint {
        usvg::Paint::Color(usvg::Color { red, green, blue }) => {
            paint.set_color_rgba8(red, green, blue, fill.opacity.to_u8())
        }
        usvg::Paint::Link(_) => {}
    }

    let rule = match fill.rule {
        usvg::FillRule::NonZero => sk::FillRule::Winding,
        usvg::FillRule::EvenOdd => sk::FillRule::EvenOdd,
    };

    (paint, rule)
}

fn convert_usvg_path(path: &usvg::PathData) -> sk::Path {
    let mut builder = sk::PathBuilder::new();
    let f = |v: f64| v as f32;
    for seg in path.iter() {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                builder.move_to(f(x), f(y));
            }
            usvg::PathSegment::LineTo { x, y } => {
                builder.line_to(f(x), f(y));
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                builder.cubic_to(f(x1), f(y1), f(x2), f(y2), f(x), f(y));
            }
            usvg::PathSegment::ClosePath => {
                builder.close();
            }
        }
    }
    builder.finish().unwrap()
}

struct WrappedPathBuilder(sk::PathBuilder);

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
