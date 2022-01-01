use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use image::{GenericImageView, Rgba};
use tiny_skia as sk;
use ttf_parser::{GlyphId, OutlineBuilder};
use usvg::FitTo;
use walkdir::WalkDir;

use typst::diag::Error;
use typst::eval::{Smart, StyleMap, Value};
use typst::font::Face;
use typst::frame::{Element, Frame, Geometry, Shape, Stroke, Text};
use typst::geom::{self, Color, Length, Paint, PathElement, RgbaColor, Size, Transform};
use typst::image::{Image, RasterImage, Svg};
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
        println!("Running {} tests", len);
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
        println!("{} / {} tests passed.", ok, len);
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
                test_part(ctx, src_path, part.into(), i, compare_ref, line, debug);
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
) -> (bool, bool, Vec<Rc<Frame>>) {
    let id = ctx.sources.provide(src_path, src);
    let source = ctx.sources.get(id);

    let (local_compare_ref, mut ref_errors) = parse_metadata(&source);
    let compare_ref = local_compare_ref.unwrap_or(compare_ref);

    let mut ok = true;
    let (frames, mut errors) = match ctx.evaluate(id) {
        Ok(module) => {
            let tree = module.into_root();
            if debug {
                println!("{:#?}", tree);
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
    tree: &RootNode,
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

        let cached = silenced(|| tree.layout(ctx));
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
            println!(
                "    Subtest {} relayout differs from clean pass on level {} ❌",
                i, level
            );
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
        "Error: {}:{}-{}:{}: {}",
        start_line, start_col, end_line, end_col, error.message
    );
}

fn draw(ctx: &Context, frames: &[Rc<Frame>], dpp: f32) -> sk::Pixmap {
    let pad = Length::pt(5.0);
    let width = 2.0 * pad + frames.iter().map(|l| l.size.x).max().unwrap_or_default();
    let height = pad + frames.iter().map(|l| l.size.y + pad).sum::<Length>();

    let pxw = (dpp * width.to_f32()) as u32;
    let pxh = (dpp * height.to_f32()) as u32;
    if pxw > 4000 || pxh > 4000 {
        panic!(
            "overlarge image: {} by {} ({:?} x {:?})",
            pxw, pxh, width, height,
        );
    }

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();
    canvas.fill(sk::Color::BLACK);

    let mut mask = sk::ClipMask::new();
    let rect = sk::Rect::from_xywh(0.0, 0.0, pxw as f32, pxh as f32).unwrap();
    let path = sk::PathBuilder::from_rect(rect);
    mask.set_path(pxw, pxh, &path, sk::FillRule::default(), false);

    let mut ts =
        sk::Transform::from_scale(dpp, dpp).pre_translate(pad.to_f32(), pad.to_f32());

    for frame in frames {
        let mut background = sk::Paint::default();
        background.set_color(sk::Color::WHITE);

        let w = frame.size.x.to_f32();
        let h = frame.size.y.to_f32();
        let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
        canvas.fill_rect(rect, &background, ts, None);

        draw_frame(&mut canvas, ts, &mask, ctx, frame, true);
        ts = ts.pre_translate(0.0, (frame.size.y + pad).to_f32());
    }

    canvas
}

fn draw_frame(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: &sk::ClipMask,
    ctx: &Context,
    frame: &Frame,
    clip: bool,
) {
    let mut storage;
    let mut mask = mask;
    if clip {
        let w = frame.size.x.to_f32();
        let h = frame.size.y.to_f32();
        let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
        let path = sk::PathBuilder::from_rect(rect).transform(ts).unwrap();
        let rule = sk::FillRule::default();
        storage = mask.clone();
        if storage.intersect_path(&path, rule, false).is_none() {
            // Fails if clipping rect is empty. In that case we just clip
            // everything by returning.
            return;
        }
        mask = &storage;
    }

    for (pos, element) in &frame.elements {
        let x = pos.x.to_f32();
        let y = pos.y.to_f32();
        let ts = ts.pre_translate(x, y);

        match *element {
            Element::Group(ref group) => {
                let ts = ts.pre_concat(convert_typst_transform(group.transform));
                draw_frame(canvas, ts, &mask, ctx, &group.frame, group.clips);
            }
            Element::Text(ref text) => {
                draw_text(canvas, ts, mask, ctx.fonts.get(text.face_id), text);
            }
            Element::Shape(ref shape) => {
                draw_shape(canvas, ts, mask, shape);
            }
            Element::Image(id, size) => {
                draw_image(canvas, ts, mask, ctx.images.get(id), size);
            }
            Element::Link(_, s) => {
                let fill = RgbaColor::new(40, 54, 99, 40).into();
                let shape = Shape::filled(Geometry::Rect(s), fill);
                draw_shape(canvas, ts, mask, &shape);
            }
        }
    }
}

fn draw_text(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: &sk::ClipMask,
    face: &Face,
    text: &Text,
) {
    let ttf = face.ttf();
    let size = text.size.to_f32();
    let units_per_em = face.units_per_em as f32;
    let pixels_per_em = text.size.to_f32() * ts.sy;
    let scale = size / units_per_em;

    let mut x = 0.0;
    for glyph in &text.glyphs {
        let glyph_id = GlyphId(glyph.id);
        let offset = x + glyph.x_offset.resolve(text.size).to_f32();
        let ts = ts.pre_translate(offset, 0.0);

        if let Some(tree) = ttf
            .glyph_svg_image(glyph_id)
            .and_then(|data| std::str::from_utf8(data).ok())
            .map(|svg| {
                let viewbox = format!("viewBox=\"0 0 {0} {0}\" xmlns", units_per_em);
                svg.replace("xmlns", &viewbox)
            })
            .and_then(|s| {
                usvg::Tree::from_str(&s, &usvg::Options::default().to_ref()).ok()
            })
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
                        canvas.fill_path(&path, &paint, fill_rule, ts, Some(mask));
                    }
                }
            }
        } else if let Some(raster) =
            ttf.glyph_raster_image(glyph_id, pixels_per_em as u16)
        {
            // TODO: Vertical alignment isn't quite right for Apple Color Emoji,
            // and maybe also for Noto Color Emoji. And: Is the size calculation
            // correct?
            let img = RasterImage::parse(&raster.data).unwrap();
            let h = text.size;
            let w = (img.width() as f64 / img.height() as f64) * h;
            let dx = (raster.x as f32) / (img.width() as f32) * size;
            let dy = (raster.y as f32) / (img.height() as f32) * size;
            let ts = ts.pre_translate(dx, -size - dy);
            draw_image(canvas, ts, mask, &Image::Raster(img), Size::new(w, h));
        } else {
            // Otherwise, draw normal outline.
            let mut builder = WrappedPathBuilder(sk::PathBuilder::new());
            if ttf.outline_glyph(glyph_id, &mut builder).is_some() {
                // Flip vertically because font design coordinate system is Y-up.
                let ts = ts.pre_scale(scale, -scale);
                let path = builder.0.finish().unwrap();
                let paint = convert_typst_paint(text.fill);
                canvas.fill_path(&path, &paint, sk::FillRule::default(), ts, Some(mask));
            }
        }

        x += glyph.x_advance.resolve(text.size).to_f32();
    }
}

fn draw_shape(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: &sk::ClipMask,
    shape: &Shape,
) {
    let path = match shape.geometry {
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
            sk::PathBuilder::from_rect(rect)
        }
        Geometry::Ellipse(size) => {
            let approx = geom::Path::ellipse(size);
            convert_typst_path(&approx)
        }
        Geometry::Line(target) => {
            let mut builder = sk::PathBuilder::new();
            builder.line_to(target.x.to_f32(), target.y.to_f32());
            builder.finish().unwrap()
        }
        Geometry::Path(ref path) => convert_typst_path(path),
    };

    if let Some(fill) = shape.fill {
        let mut paint = convert_typst_paint(fill);
        if matches!(shape.geometry, Geometry::Rect(_)) {
            paint.anti_alias = false;
        }

        let rule = sk::FillRule::default();
        canvas.fill_path(&path, &paint, rule, ts, Some(mask));
    }

    if let Some(Stroke { paint, thickness }) = shape.stroke {
        let paint = convert_typst_paint(paint);
        let mut stroke = sk::Stroke::default();
        stroke.width = thickness.to_f32();
        canvas.stroke_path(&path, &paint, &stroke, ts, Some(mask));
    }
}

fn draw_image(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: &sk::ClipMask,
    img: &Image,
    size: Size,
) {
    let view_width = size.x.to_f32();
    let view_height = size.y.to_f32();

    let pixmap = match img {
        Image::Raster(img) => {
            let w = img.buf.width();
            let h = img.buf.height();
            let mut pixmap = sk::Pixmap::new(w, h).unwrap();
            for ((_, _, src), dest) in img.buf.pixels().zip(pixmap.pixels_mut()) {
                let Rgba([r, g, b, a]) = src;
                *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
            }
            pixmap
        }
        Image::Svg(Svg(tree)) => {
            let size = tree.svg_node().size;
            let aspect = (size.width() / size.height()) as f32;
            let scale = ts.sx.max(ts.sy);
            let w = (scale * view_width.max(aspect * view_height)).ceil() as u32;
            let h = ((w as f32) / aspect).ceil() as u32;
            let mut pixmap = sk::Pixmap::new(w, h).unwrap();
            resvg::render(&tree, FitTo::Size(w, h), pixmap.as_mut());
            pixmap
        }
    };

    let scale_x = view_width / pixmap.width() as f32;
    let scale_y = view_height / pixmap.height() as f32;

    let mut paint = sk::Paint::default();
    paint.shader = sk::Pattern::new(
        pixmap.as_ref(),
        sk::SpreadMode::Pad,
        sk::FilterQuality::Bilinear,
        1.0,
        sk::Transform::from_scale(scale_x, scale_y),
    );

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height).unwrap();
    canvas.fill_rect(rect, &paint, ts, Some(mask));
}

fn convert_typst_transform(transform: Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_f32(),
        ty.to_f32(),
    )
}

fn convert_typst_paint(paint: Paint) -> sk::Paint<'static> {
    let Paint::Solid(Color::Rgba(c)) = paint;
    let mut paint = sk::Paint::default();
    paint.set_color_rgba8(c.r, c.g, c.b, c.a);
    paint.anti_alias = true;
    paint
}

fn convert_typst_path(path: &geom::Path) -> sk::Path {
    let mut builder = sk::PathBuilder::new();
    for elem in &path.0 {
        match elem {
            PathElement::MoveTo(p) => {
                builder.move_to(p.x.to_f32(), p.y.to_f32());
            }
            PathElement::LineTo(p) => {
                builder.line_to(p.x.to_f32(), p.y.to_f32());
            }
            PathElement::CubicTo(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x.to_f32(),
                    p1.y.to_f32(),
                    p2.x.to_f32(),
                    p2.y.to_f32(),
                    p3.x.to_f32(),
                    p3.y.to_f32(),
                );
            }
            PathElement::ClosePath => {
                builder.close();
            }
        };
    }
    builder.finish().unwrap()
}

fn convert_usvg_transform(transform: usvg::Transform) -> sk::Transform {
    let usvg::Transform { a, b, c, d, e, f } = transform;
    sk::Transform::from_row(a as _, b as _, c as _, d as _, e as _, f as _)
}

fn convert_usvg_fill(fill: &usvg::Fill) -> (sk::Paint<'static>, sk::FillRule) {
    let mut paint = sk::Paint::default();
    paint.anti_alias = true;

    if let usvg::Paint::Color(usvg::Color { red, green, blue, alpha: _ }) = fill.paint {
        paint.set_color_rgba8(red, green, blue, fill.opacity.to_u8())
    }

    let rule = match fill.rule {
        usvg::FillRule::NonZero => sk::FillRule::Winding,
        usvg::FillRule::EvenOdd => sk::FillRule::EvenOdd,
    };

    (paint, rule)
}

fn convert_usvg_path(path: &usvg::PathData) -> sk::Path {
    let mut builder = sk::PathBuilder::new();
    for seg in path.iter() {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                builder.move_to(x as _, y as _);
            }
            usvg::PathSegment::LineTo { x, y } => {
                builder.line_to(x as _, y as _);
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                builder.cubic_to(x1 as _, y1 as _, x2 as _, y2 as _, x as _, y as _);
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

/// Additional methods for [`Length`].
trait LengthExt {
    /// Convert an em length to a number of points.
    fn to_f32(self) -> f32;
}

impl LengthExt for Length {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
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
