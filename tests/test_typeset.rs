use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufWriter;
use std::rc::Rc;

use fontdock::fs::{FsIndex, FsProvider};
use fontdock::FontLoader;
use futures_executor::block_on;
use raqote::{DrawTarget, PathBuilder, SolidSource, Source, Transform, Vector};
use ttf_parser::OutlineBuilder;

use typstc::export::pdf;
use typstc::font::{DynProvider, SharedFontLoader};
use typstc::geom::{Size, Value4};
use typstc::layout::elements::{LayoutElement, Shaped};
use typstc::layout::MultiLayout;
use typstc::length::Length;
use typstc::paper::PaperClass;
use typstc::style::PageStyle;
use typstc::Typesetter;

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
            filtered.push((name, src));
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

    let (descriptors, files) = index.clone().into_vecs();
    let provider = FsProvider::new(files.clone());
    let dynamic = Box::new(provider) as Box<DynProvider>;
    let loader = FontLoader::new(dynamic, descriptors);
    let loader = Rc::new(RefCell::new(loader));
    let mut typesetter = Typesetter::new(loader.clone());

    typesetter.set_page_style(PageStyle {
        class: PaperClass::Custom,
        size: Size::with_all(Length::pt(250.0).as_raw()),
        margins: Value4::with_all(None),
    });

    for (name, src) in filtered {
        test(&name, &src, &mut typesetter, &loader)
    }
}

fn test(
    name: &str,
    src: &str,
    typesetter: &mut Typesetter,
    loader: &SharedFontLoader,
) {
    println!("Testing {}.", name);

    let typeset = block_on(typesetter.typeset(src));
    let layouts = typeset.output;
    let mut feedback = typeset.feedback;

    feedback.diagnostics.sort();
    for diagnostic in feedback.diagnostics {
        println!(
            "  {:?} {:?}: {}",
            diagnostic.v.level, diagnostic.span, diagnostic.v.message,
        );
    }

    let png_path = format!("{}/{}.png", OUT_DIR, name);
    render(&layouts, &loader, 3.0).write_png(png_path).unwrap();

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
            self.filter.is_empty()
                || self.filter.iter().any(|p| name.contains(p))
        }
    }
}

fn render(
    layouts: &MultiLayout,
    loader: &SharedFontLoader,
    scale: f64,
) -> DrawTarget {
    let pad = scale * 10.0;
    let width = 2.0 * pad + layouts.iter()
        .map(|l| scale * l.size.x)
        .max_by(|a, b| a.partial_cmp(&b).unwrap())
        .unwrap()
        .round();

    let height = pad + layouts.iter()
        .map(|l| scale * l.size.y + pad)
        .sum::<f64>()
        .round();

    let mut surface = DrawTarget::new(width as i32, height as i32);
    surface.clear(BLACK);

    let mut offset = Size::new(pad, pad);
    for layout in layouts {
        surface.fill_rect(
            offset.x as f32,
            offset.y as f32,
            (scale * layout.size.x) as f32,
            (scale * layout.size.y) as f32,
            &Source::Solid(WHITE),
            &Default::default(),
        );

        for &(pos, ref element) in &layout.elements.0 {
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

        offset.y += scale * layout.size.y + pad;
    }

    surface
}

fn render_shaped(
    surface: &mut DrawTarget,
    loader: &SharedFontLoader,
    shaped: &Shaped,
    pos: Size,
    scale: f64,
) {
    let loader = loader.borrow();
    let face = loader.get_loaded(shaped.face);

    for (&glyph, &offset) in shaped.glyphs.iter().zip(&shaped.offsets) {
        let mut builder = WrappedPathBuilder(PathBuilder::new());
        face.outline_glyph(glyph, &mut builder);
        let path = builder.0.finish();

        let units_per_em = face.units_per_em().unwrap_or(1000);
        let s = scale * (shaped.size / units_per_em as f64);
        let x = pos.x + scale * offset;
        let y = pos.y + scale * shaped.size;

        let t = Transform::create_scale(s as f32, -s as f32)
            .post_translate(Vector::new(x as f32, y as f32));

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
