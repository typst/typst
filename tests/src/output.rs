use tiny_skia as sk;
use typst::Document;
use typst::diag::SourceResult;
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Transform};
use typst::visualize::Color;
use typst_html::HtmlDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};

use crate::collect::{Test, TestOutput};

pub struct Hash([u8; 40]);

/// An output type we can test.
pub trait OutputType: Sized {
    /// The document type this output requires.
    type Doc: Document + Clone;
    /// The type that represents live output.
    type Live;

    /// The subdirectory in `ref` and `store`.
    const DIR: &str;
    /// The file extension for live output.
    const EXTENSION: &str;
    /// The test output type.
    const OUTPUT: TestOutput;

    /// Whether the test output is trivial and needs no reference output.
    fn is_skippable(_doc: &Self::Doc, _live: &Self::Live) -> Result<bool, ()> {
        Ok(false)
    }

    /// Produces the live output.
    fn make_live(test: &Test, doc: &mut Self::Doc) -> SourceResult<Self::Live>;

    /// Converts the live output to bytes that can be saved to disk.
    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]>;
}

pub trait FileOutputType: OutputType {
    /// Produces the reference output from the live output.
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]>;

    /// Checks whether the reference output matches.
    fn matches(old: &[u8], new: &Self::Live) -> bool;
}

pub trait HashOutputType: OutputType {
    /// Produces the reference output from the live output.
    fn make_hash(live: Self::Live) -> Hash;
}

pub struct Render;

impl OutputType for Render {
    type Doc = PagedDocument;
    type Live = tiny_skia::Pixmap;

    const DIR: &str = "render";
    const EXTENSION: &str = "png";
    const OUTPUT: TestOutput = TestOutput::RENDER;

    fn is_skippable(doc: &Self::Doc, _: &Self::Live) -> Result<bool, ()> {
        is_empty_paged_document(doc)
    }

    fn make_live(_: &Test, doc: &mut Self::Doc) -> SourceResult<Self::Live> {
        Ok(render(doc, 1.0))
    }

    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        // Save live version, possibly rerendering if different scale is
        // requested.
        let mut pixmap_live = live;
        let slot;
        let scale = crate::ARGS.scale;
        if scale != 1.0 {
            slot = render(doc, scale);
            pixmap_live = &slot;
        }
        pixmap_live.encode_png().unwrap()
    }
}

impl FileOutputType for Render {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        let opts = oxipng::Options::max_compression();
        let data = live.encode_png().unwrap();
        oxipng::optimize_from_memory(&data, &opts).unwrap()
    }

    fn matches(old: &[u8], new: &Self::Live) -> bool {
        let old_pixmap = sk::Pixmap::decode_png(old).unwrap();
        approx_equal(&old_pixmap, new)
    }
}

pub struct Pdf;

impl OutputType for Pdf {
    type Doc = PagedDocument;
    type Live = Vec<u8>;

    const DIR: &str = "pdf";
    const EXTENSION: &str = "pdf";
    const OUTPUT: TestOutput = TestOutput::PDF;

    fn make_live(_: &Test, doc: &mut Self::Doc) -> SourceResult<Self::Live> {
        typst_pdf::pdf(doc, &PdfOptions::default())
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Pdf {
    fn make_hash(live: Self::Live) -> Hash {
        todo!()
    }
}

pub struct Pdftags;

impl OutputType for Pdftags {
    type Doc = PagedDocument;
    type Live = String;

    const DIR: &str = "pdftags";
    const EXTENSION: &str = "yml";
    const OUTPUT: TestOutput = TestOutput::PDFTAGS;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(test: &Test, doc: &mut PagedDocument) -> SourceResult<Self::Live> {
        let standards = if test.attrs.pdf_ua {
            if doc.info.title.is_none() {
                doc.info.title = Some("<test>".into());
            }
            PdfStandards::new(&[PdfStandard::Ua_1]).unwrap()
        } else {
            PdfStandards::default()
        };
        let options = PdfOptions { standards, ..Default::default() };
        typst_pdf::pdf_tags(&doc, &options)
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl FileOutputType for Pdftags {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn matches(old: &[u8], new: &Self::Live) -> bool {
        old == new.as_bytes()
    }
}

pub struct Svg;

impl OutputType for Svg {
    type Doc = PagedDocument;
    type Live = String;

    const DIR: &str = "svg";
    const EXTENSION: &str = "svg";
    const OUTPUT: TestOutput = TestOutput::SVG;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &mut Self::Doc) -> SourceResult<Self::Live> {
        Ok(typst_svg::svg_merged(doc, Abs::pt(5.0)))
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Svg {
    fn make_hash(live: Self::Live) -> Hash {
        todo!()
    }
}

pub struct Html;

impl OutputType for Html {
    type Doc = HtmlDocument;
    type Live = String;

    const DIR: &str = "html";
    const EXTENSION: &str = "html";
    const OUTPUT: TestOutput = TestOutput::HTML;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &mut Self::Doc) -> SourceResult<Self::Live> {
        typst_html::html(doc)
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl FileOutputType for Html {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn matches(old: &[u8], new: &Self::Live) -> bool {
        old == new.as_bytes()
    }
}

/// Whether rendering of this document can be skipped.
fn is_empty_paged_document(doc: &PagedDocument) -> Result<bool, ()> {
    fn skippable_frame(frame: &Frame) -> bool {
        frame.items().all(|(_, item)| match item {
            FrameItem::Group(group) => skippable_frame(&group.frame),
            FrameItem::Tag(_) => true,
            _ => false,
        })
    }

    match doc.pages.as_slice() {
        [] => Err(()),
        [page] => Ok(page.frame.width().approx_eq(Abs::pt(120.0))
            && page.frame.height().approx_eq(Abs::pt(20.0))
            && page.fill.is_auto()
            && skippable_frame(&page.frame)),
        _ => Ok(false),
    }
}

/// Draw all frames into one image with padding in between.
fn render(document: &PagedDocument, pixel_per_pt: f32) -> sk::Pixmap {
    for page in &document.pages {
        let limit = Abs::cm(100.0);
        if page.frame.width() > limit || page.frame.height() > limit {
            panic!("overlarge frame: {:?}", page.frame.size());
        }
    }

    let gap = Abs::pt(1.0);
    let mut pixmap =
        typst_render::render_merged(document, pixel_per_pt, gap, Some(Color::BLACK));

    let gap = (pixel_per_pt * gap.to_pt() as f32).round();

    let mut y = 0.0;
    for page in &document.pages {
        let ts =
            sk::Transform::from_scale(pixel_per_pt, pixel_per_pt).post_translate(0.0, y);
        render_links(&mut pixmap, ts, &page.frame);
        y += (pixel_per_pt * page.frame.height().to_pt() as f32).round().max(1.0) + gap;
    }

    pixmap
}

/// Draw extra boxes for links so we can see whether they are there.
fn render_links(canvas: &mut sk::Pixmap, ts: sk::Transform, frame: &Frame) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *item {
            FrameItem::Group(ref group) => {
                let ts = ts.pre_concat(to_sk_transform(&group.transform));
                render_links(canvas, ts, &group.frame);
            }
            FrameItem::Link(_, size) => {
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

/// Whether two pixel images are approximately equal.
fn approx_equal(a: &sk::Pixmap, b: &sk::Pixmap) -> bool {
    a.width() == b.width()
        && a.height() == b.height()
        && a.data().iter().zip(b.data()).all(|(&a, &b)| a.abs_diff(b) <= 1)
}

/// Convert a Typst transform to a tiny-skia transform.
fn to_sk_transform(transform: &Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = *transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_pt() as f32,
        ty.to_pt() as f32,
    )
}
