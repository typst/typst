use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsSource};
use raqote::{DrawTarget, PathBuilder, SolidSource, Source, Transform, Vector};
use ttf_parser::OutlineBuilder;

use typstc::diag::{Feedback, Pass};
use typstc::eval::State;
use typstc::export::pdf;
use typstc::font::{FontLoader, SharedFontLoader};
use typstc::geom::{Length, Point};
use typstc::layout::{BoxLayout, LayoutElement};
use typstc::parse::LineMap;
use typstc::shaping::Shaped;
use typstc::typeset;

const TEST_DIR: &str = "tests";
const OUT_DIR: &str = "tests/out";
const FONT_DIR: &str = "fonts";

const BLACK: SolidSource = SolidSource { r: 0, g: 0, b: 0, a: 255 };
const WHITE: SolidSource = SolidSource { r: 255, g: 255, b: 255, a: 255 };

fn main() {
    let filter = TestFilter::new(env::args().skip(1));
    let mut filtered = Vec::new();

    for entry in fs::read_dir(TEST_DIR).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(OsStr::new("typ")) {
            continue;
        }

        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        if filter.matches(&name) {
            let src = fs::read_to_string(&path).unwrap();
            filtered.push((name, path, src));
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

    fs::create_dir_all(OUT_DIR).unwrap();

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (files, descriptors) = index.into_vecs();
    let loader = Rc::new(RefCell::new(FontLoader::new(
        Box::new(FsSource::new(files)),
        descriptors,
    )));

    for (name, path, src) in filtered {
        test(&name, &src, &path, &loader)
    }
}

fn test(name: &str, src: &str, src_path: &Path, loader: &SharedFontLoader) {
    println!("Testing {}.", name);

    let state = State::default();
    let Pass {
        output: layouts,
        feedback: Feedback { mut diags, .. },
    } = typeset(&src, state, Rc::clone(loader));

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

    let loader = loader.borrow();

    let png_path = format!("{}/{}.png", OUT_DIR, name);
    let surface = render(&layouts, &loader, 3.0);
    surface.write_png(png_path).unwrap();

    let pdf_path = format!("{}/{}.pdf", OUT_DIR, name);
    let file = BufWriter::new(File::create(pdf_path).unwrap());
    pdf::export(&layouts, &loader, file).unwrap();
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

fn render(layouts: &[BoxLayout], loader: &FontLoader, scale: f64) -> DrawTarget {
    let pad = Length::pt(scale * 10.0);
    let width = 2.0 * pad
        + layouts
            .iter()
            .map(|l| scale * l.size.width)
            .max_by(|a, b| a.partial_cmp(&b).unwrap())
            .unwrap();

    let height =
        pad + layouts.iter().map(|l| scale * l.size.height + pad).sum::<Length>();

    let int_width = width.to_pt().round() as i32;
    let int_height = height.to_pt().round() as i32;
    let mut surface = DrawTarget::new(int_width, int_height);
    surface.clear(BLACK);

    let mut offset = Point::new(pad, pad);
    for layout in layouts {
        surface.fill_rect(
            offset.x.to_pt() as f32,
            offset.y.to_pt() as f32,
            (scale * layout.size.width).to_pt() as f32,
            (scale * layout.size.height).to_pt() as f32,
            &Source::Solid(WHITE),
            &Default::default(),
        );

        for &(pos, ref element) in &layout.elements {
            match element {
                LayoutElement::Text(shaped) => render_shaped(
                    &mut surface,
                    loader,
                    shaped,
                    scale * pos + offset,
                    scale,
                ),
            }
        }

        offset.y += scale * layout.size.height + pad;
    }

    surface
}

fn render_shaped(
    surface: &mut DrawTarget,
    loader: &FontLoader,
    shaped: &Shaped,
    pos: Point,
    scale: f64,
) {
    let face = loader.get_loaded(shaped.face).get();

    for (&glyph, &offset) in shaped.glyphs.iter().zip(&shaped.offsets) {
        let mut builder = WrappedPathBuilder(PathBuilder::new());
        face.outline_glyph(glyph, &mut builder);
        let path = builder.0.finish();

        let units_per_em = face.units_per_em().unwrap_or(1000);
        let s = scale * (shaped.font_size / units_per_em as f64);
        let x = pos.x + scale * offset;
        let y = pos.y + scale * shaped.font_size;

        let t = Transform::create_scale(s.to_pt() as f32, -s.to_pt() as f32)
            .post_translate(Vector::new(x.to_pt() as f32, y.to_pt() as f32));

        surface.fill(
            &path.transform(&t),
            &Source::Solid(SolidSource { r: 0, g: 0, b: 0, a: 255 }),
            &Default::default(),
        )
    }
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
