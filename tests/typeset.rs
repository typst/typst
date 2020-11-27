use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::Path;
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsSource};
use image::{DynamicImage, GenericImageView, Rgba};
use memmap::Mmap;
use tiny_skia::{
    Canvas, Color, ColorU8, FillRule, FilterQuality, Paint, PathBuilder, Pattern, Pixmap,
    Rect, SpreadMode, Transform,
};
use ttf_parser::OutlineBuilder;

use typst::diag::{Feedback, Pass};
use typst::env::{Env, ResourceLoader, SharedEnv};
use typst::eval::State;
use typst::export::pdf;
use typst::font::FontLoader;
use typst::geom::{Length, Point};
use typst::layout::{BoxLayout, ImageElement, LayoutElement};
use typst::parse::LineMap;
use typst::shaping::Shaped;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "typ";
const PDF_DIR: &str = "pdf";
const PNG_DIR: &str = "png";
const REF_DIR: &str = "ref";

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
        let pdf_path = Path::new(PDF_DIR).join(&name).with_extension("pdf");
        let png_path = Path::new(PNG_DIR).join(&name).with_extension("png");
        let ref_path = Path::new(REF_DIR).join(&name).with_extension("png");

        if filter.matches(&name) {
            filtered.push((name, src_path, pdf_path, png_path, ref_path));
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

    fs::create_dir_all(PDF_DIR).unwrap();
    fs::create_dir_all(PNG_DIR).unwrap();

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (files, descriptors) = index.into_vecs();
    let env = Rc::new(RefCell::new(Env {
        fonts: FontLoader::new(Box::new(FsSource::new(files)), descriptors),
        resources: ResourceLoader::new(),
    }));

    let mut ok = true;

    for (name, src_path, pdf_path, png_path, ref_path) in filtered {
        print!("Testing {}.", name);
        test(&src_path, &pdf_path, &png_path, &env);

        let png_file = File::open(&png_path).unwrap();
        let ref_file = match File::open(&ref_path) {
            Ok(file) => file,
            Err(_) => {
                println!(" Failed to open reference image. ❌");
                ok = false;
                continue;
            }
        };

        let a = unsafe { Mmap::map(&png_file).unwrap() };
        let b = unsafe { Mmap::map(&ref_file).unwrap() };

        if *a != *b {
            println!(" Does not match reference image. ❌");
            ok = false;
        } else {
            println!(" Okay. ✔");
        }
    }

    if !ok {
        std::process::exit(1);
    }
}

fn test(src_path: &Path, pdf_path: &Path, png_path: &Path, env: &SharedEnv) {
    let src = fs::read_to_string(src_path).unwrap();
    let state = State::default();
    let Pass {
        output: layouts,
        feedback: Feedback { mut diags, .. },
    } = typeset(&src, Rc::clone(env), state);

    if !diags.is_empty() {
        diags.sort();

        let map = LineMap::new(&src);
        for diag in diags {
            let span = diag.span;
            let start = map.location(span.start);
            let end = map.location(span.end);
            println!(
                "  {}: {}:{}-{}: {}",
                diag.v.level,
                src_path.display(),
                start,
                end,
                diag.v.message,
            );
        }
    }

    let env = env.borrow();
    let canvas = draw(&layouts, &env, 2.0);
    canvas.pixmap.save_png(png_path).unwrap();

    let pdf_data = pdf::export(&layouts, &env);
    fs::write(pdf_path, pdf_data).unwrap();
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

fn draw(layouts: &[BoxLayout], env: &Env, pixel_per_pt: f32) -> Canvas {
    let pad = Length::pt(5.0);

    let height = pad + layouts.iter().map(|l| l.size.height + pad).sum::<Length>();
    let width = 2.0 * pad
        + layouts
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
    for layout in layouts {
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);

        canvas.fill_rect(
            Rect::from_xywh(
                origin.x.to_pt() as f32,
                origin.y.to_pt() as f32,
                layout.size.width.to_pt() as f32,
                layout.size.height.to_pt() as f32,
            )
            .unwrap(),
            &paint,
        );

        for &(pos, ref element) in &layout.elements {
            let pos = origin + pos;
            match element {
                LayoutElement::Text(shaped) => {
                    draw_text(&mut canvas, pos, env, shaped);
                }
                LayoutElement::Image(image) => {
                    draw_image(&mut canvas, pos, env, image);
                }
            }
        }

        origin.y += layout.size.height + pad;
    }

    canvas
}

fn draw_text(canvas: &mut Canvas, pos: Point, env: &Env, shaped: &Shaped) {
    let face = env.fonts.get_loaded(shaped.face).get();

    for (&glyph, &offset) in shaped.glyphs.iter().zip(&shaped.offsets) {
        let units_per_em = face.units_per_em().unwrap_or(1000);

        let x = (pos.x + offset).to_pt() as f32;
        let y = (pos.y + shaped.font_size).to_pt() as f32;
        let scale = (shaped.font_size / units_per_em as f64).to_pt() as f32;

        let mut builder = WrappedPathBuilder(PathBuilder::new());
        face.outline_glyph(glyph, &mut builder);

        let path = builder.0.finish().unwrap();
        let placed = path
            .transform(&Transform::from_row(scale, 0.0, 0.0, -scale, x, y).unwrap())
            .unwrap();

        let mut paint = Paint::default();
        paint.anti_alias = true;

        canvas.fill_path(&placed, &paint, FillRule::default());
    }
}

fn draw_image(canvas: &mut Canvas, pos: Point, env: &Env, image: &ImageElement) {
    let buf = env.resources.get_loaded::<DynamicImage>(image.resource);

    let mut pixmap = Pixmap::new(buf.width(), buf.height()).unwrap();
    for ((_, _, src), dest) in buf.pixels().zip(pixmap.pixels_mut()) {
        let Rgba([r, g, b, a]) = src;
        *dest = ColorU8::from_rgba(r, g, b, a).premultiply();
    }

    let view_width = image.size.width.to_pt() as f32;
    let view_height = image.size.height.to_pt() as f32;

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
