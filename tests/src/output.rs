use std::fmt::Display;
use std::str::FromStr;

use ecow::EcoString;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use tiny_skia as sk;
use typst::diag::{At, SourceResult, StrResult, bail};
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Transform};
use typst::visualize::Color;
use typst_html::HtmlDocument;
use typst_pdf::{PdfOptions, PdfStandards};
use typst_syntax::Span;

use crate::collect::{Test, TestOutput};
use crate::pdftags;

/// A map from a test name to the corresponding reference hash.
#[derive(Default)]
pub struct HashedRefs {
    /// Whether a reference hash has been added/removed/updated and the hash
    /// file on disk should be updated.
    pub changed: bool,
    refs: IndexMap<EcoString, HashedRef, FxBuildHasher>,
}

impl HashedRefs {
    /// Get the reference hash for a test.
    pub fn get(&self, name: &str) -> Option<HashedRef> {
        self.refs.get(name).copied()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// Remove a reference hash.
    pub fn remove(&mut self, name: &str) {
        self.changed = true;
        self.refs.shift_remove(name);
    }

    /// Update a reference hash.
    pub fn update(&mut self, name: EcoString, hashed_ref: HashedRef) {
        self.changed = true;
        self.refs.insert(name, hashed_ref);
    }

    /// Sort the reference hashes lexicographically.
    pub fn sort(&mut self) {
        if !self.refs.keys().is_sorted() {
            self.changed = true;
            self.refs.sort_keys();
        }
    }

    /// The names of all tests in this map.
    pub fn names(&self) -> impl Iterator<Item = &EcoString> {
        self.refs.keys()
    }
}

impl Display for HashedRefs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, hash) in self.refs.iter() {
            writeln!(f, "{hash} {name}")?;
        }
        Ok(())
    }
}

impl FromStr for HashedRefs {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let refs = s
            .lines()
            .map(|line| {
                let mut parts = line.split_whitespace();
                let Some(hash) = parts.next() else { bail!("found empty line") };
                let hash = hash.parse()?;

                let Some(name) = parts.next() else { bail!("missing test name") };

                if parts.next().is_some() {
                    bail!("found trailing characters");
                }

                Ok((name.into(), hash))
            })
            .collect::<StrResult<IndexMap<_, _, _>>>()?;
        Ok(HashedRefs { changed: false, refs })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HashedRef(u128);

impl Display for HashedRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl FromStr for HashedRef {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            bail!("invalid hash: hexadecimal length must be 32");
        }
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("invalid hash: not a valid hexadecimal digit");
        }
        let val = u128::from_str_radix(s, 16).unwrap();
        Ok(HashedRef(val))
    }
}

/// An output type we can test.
pub trait OutputType: Sized {
    /// The document type this output requires.
    type Doc;
    /// The type that represents live output.
    type Live;

    /// The test output format.
    const OUTPUT: TestOutput;

    /// Whether the test output is trivial and needs no reference output.
    fn is_skippable(_doc: &Self::Doc, _live: &Self::Live) -> Result<bool, ()> {
        Ok(false)
    }

    /// Produces the live output.
    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live>;

    /// Converts the live output to bytes that can be saved to disk.
    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]>;
}

/// An output type that produces file references.
pub trait FileOutputType: OutputType {
    /// Produces the reference output from the live output.
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]>;

    /// Checks whether the reference output matches.
    fn matches(old: &[u8], new: &Self::Live) -> bool;
}

/// An output type that produces hashed references.
pub trait HashOutputType: OutputType {
    /// The index into the [`crate::run::HASHES`] array.
    const INDEX: usize;

    /// Produces the reference output from the live output.
    fn make_hash(live: &Self::Live) -> HashedRef;
}

pub struct Render;

impl OutputType for Render {
    type Doc = PagedDocument;
    type Live = tiny_skia::Pixmap;

    const OUTPUT: TestOutput = TestOutput::Render;

    fn is_skippable(doc: &Self::Doc, _: &Self::Live) -> Result<bool, ()> {
        is_empty_paged_document(doc)
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
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

    const OUTPUT: TestOutput = TestOutput::Pdf;

    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        let standards = PdfStandards::new(test.attrs.pdf_standard.as_slice()).unwrap();
        let options = PdfOptions { standards, ..Default::default() };
        typst_pdf::pdf(doc, &options)
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Pdf {
    const INDEX: usize = 0;

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }
}

pub struct Pdftags;

impl OutputType for Pdftags {
    type Doc = Vec<u8>;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Pdftags;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        pdftags::format(doc).at(Span::detached())
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

    const OUTPUT: TestOutput = TestOutput::Svg;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        Ok(typst_svg::svg_merged(doc, Abs::pt(5.0)))
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Svg {
    const INDEX: usize = 1;

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }
}

pub struct Html;

impl OutputType for Html {
    type Doc = HtmlDocument;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Html;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
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
