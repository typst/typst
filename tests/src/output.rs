use std::fmt::Display;
use std::option::Option;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use hayro::{FontData, FontQuery, InterpreterSettings, StandardFont};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use tiny_skia as sk;
use typst::Document;
use typst::diag::{At, SourceResult, StrResult, bail};
use typst::foundations::{Content, SequenceElem};
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Transform};
use typst::model::ParbreakElem;
use typst::text::SpaceElem;
use typst::visualize::Color;
use typst_html::HtmlDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};
use typst_syntax::Span;

use crate::collect::{Test, TestOutput, TestTarget};
use crate::report::{DiffKind, File, Old, ReportFile};
use crate::{pdftags, report};

pub trait TestDocument: Document {
    /// The target of the document.
    const TARGET: TestTarget;
}

impl TestDocument for PagedDocument {
    const TARGET: TestTarget = TestTarget::Paged;
}

impl TestDocument for HtmlDocument {
    const TARGET: TestTarget = TestTarget::Html;
}

/// A map from a test name to the corresponding reference hash.
#[derive(Default)]
pub struct HashedRefs {
    refs: IndexMap<EcoString, HashedRef, FxBuildHasher>,
}

impl HashedRefs {
    pub fn parse_line(line: &str) -> StrResult<(EcoString, HashedRef)> {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { bail!("found empty line") };
        let hash = hash.parse()?;

        let Some(name) = parts.next() else { bail!("missing test name") };

        if parts.next().is_some() {
            bail!("found trailing characters");
        }

        Ok((name.into(), hash))
    }

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
        self.refs.shift_remove(name);
    }

    /// Update a reference hash.
    pub fn update(&mut self, name: EcoString, hashed_ref: HashedRef) {
        self.refs.insert(name, hashed_ref);
    }

    /// Sort the reference hashes lexicographically.
    pub fn sort(&mut self) {
        self.refs.sort_keys();
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
        let refs = s.lines().map(HashedRefs::parse_line).collect::<StrResult<_>>()?;
        Ok(HashedRefs { refs })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    fn is_empty(doc: &Self::Doc, live: &Self::Live) -> bool;

    /// Produces the live output.
    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live>;

    /// Converts the live output to bytes that can be saved to disk.
    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]>;

    /// Produce a hash from the live output.
    fn make_hash(live: &Self::Live) -> HashedRef;

    /// Generate data necessary to make a HTML diff.
    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile;
}

/// An output type that produces file references.
pub trait FileOutputType: OutputType {
    /// Produces the reference output from the live output.
    fn save_ref(live: &Self::Live) -> impl AsRef<[u8]>;

    /// Checks whether the reference output matches.
    fn matches(old: &[u8], new: &Self::Live) -> bool;
}

/// An output type that produces hashed references.
pub trait HashOutputType: OutputType {
    /// The index into the shared `hashes` array.
    const INDEX: usize;
}

/// The [`HashOutputType`]s [`OutputType::OUTPUT`] in an array corresponding to
/// the [`HashOutputType::INDEX`].
///
/// NOTE: This has to be kept in sync with the [`HashOutputType::INDEX`].
pub const HASH_OUTPUTS: [TestOutput; 2] = [TestOutput::Pdf, TestOutput::Svg];

pub struct Render;

impl OutputType for Render {
    type Doc = PagedDocument;
    type Live = tiny_skia::Pixmap;

    const OUTPUT: TestOutput = TestOutput::Render;

    fn is_empty(doc: &Self::Doc, _: &Self::Live) -> bool {
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

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live.data()))
    }

    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile {
        let diffs = [image_diff(a, b, "png")];
        file_report(Self::OUTPUT, a, b, diffs)
    }
}

impl FileOutputType for Render {
    fn save_ref(live: &Self::Live) -> impl AsRef<[u8]> {
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

    fn is_empty(doc: &Self::Doc, _: &Self::Live) -> bool {
        is_empty_paged_document(doc)
    }

    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        // Always run the default PDF export and PDF/UA-1 export, to detect
        // crashes, since there are quite a few different code paths involved.
        // If another standard is specified in the test, run that as well.
        let default_pdf = generate_pdf(doc, None);
        let ua1_pdf = generate_pdf(doc, Some(PdfStandard::Ua_1));
        match test.attrs.pdf_standard {
            Some(PdfStandard::Ua_1) => ua1_pdf,
            Some(other) => generate_pdf(doc, Some(other)),
            None => default_pdf,
        }
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }

    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile {
        // TODO: PDF plain text diffs.
        let mut svg_buf_a = String::new();
        let mut svg_buf_b = String::new();
        let svg_a = a.map(|(_, old)| {
            old.map(|bytes| {
                svg_buf_a = pdf_to_svg(bytes);
                svg_buf_a.as_bytes()
            })
        });
        let svg_b = b.map(|(_, bytes)| {
            svg_buf_b = pdf_to_svg(bytes);
            svg_buf_b.as_bytes()
        });
        let diffs = [DiffKind::Image(report::image_diff(svg_a, svg_b, "svg+xml"))];
        file_report(Self::OUTPUT, a, b, diffs)
    }
}

fn pdf_to_svg(bytes: &[u8]) -> String {
    let pdf = hayro_syntax::Pdf::new(Arc::new(bytes.to_vec())).unwrap();
    let select_standard_font = move |font: StandardFont| -> Option<(FontData, u32)> {
        let bytes = match font {
            StandardFont::Helvetica => typst_assets::pdf::SANS,
            StandardFont::HelveticaBold => typst_assets::pdf::SANS_BOLD,
            StandardFont::HelveticaOblique => typst_assets::pdf::SANS_ITALIC,
            StandardFont::HelveticaBoldOblique => typst_assets::pdf::SANS_BOLD_ITALIC,
            StandardFont::Courier => typst_assets::pdf::FIXED,
            StandardFont::CourierBold => typst_assets::pdf::FIXED_BOLD,
            StandardFont::CourierOblique => typst_assets::pdf::FIXED_ITALIC,
            StandardFont::CourierBoldOblique => typst_assets::pdf::FIXED_BOLD_ITALIC,
            StandardFont::TimesRoman => typst_assets::pdf::SERIF,
            StandardFont::TimesBold => typst_assets::pdf::SERIF_BOLD,
            StandardFont::TimesItalic => typst_assets::pdf::SERIF_ITALIC,
            StandardFont::TimesBoldItalic => typst_assets::pdf::SERIF_BOLD_ITALIC,
            StandardFont::ZapfDingBats => typst_assets::pdf::DING_BATS,
            StandardFont::Symbol => typst_assets::pdf::SYMBOL,
        };
        Some((Arc::new(bytes), 0))
    };

    let interpreter_settings = InterpreterSettings {
        font_resolver: Arc::new(move |query| match query {
            FontQuery::Standard(s) => select_standard_font(*s),
            FontQuery::Fallback(f) => select_standard_font(f.pick_standard_font()),
        }),
        warning_sink: Arc::new(|_| {}),
    };

    let mut svg = hayro_svg::convert(&pdf.pages()[0], &interpreter_settings);

    // Insert a white background, since PDFs don't set a background by default.
    let pos = svg.find(">").expect("end of opening `<svg>` tag");
    svg.insert_str(
        pos + 1,
        r#"<rect x="0" y="0" width="100%" height="100%" fill="white"/>"#,
    );

    svg
}

impl HashOutputType for Pdf {
    const INDEX: usize = 0;
}

fn generate_pdf(
    doc: &PagedDocument,
    standard: Option<PdfStandard>,
) -> SourceResult<Vec<u8>> {
    let standards = PdfStandards::new(standard.as_slice()).unwrap();
    let options = PdfOptions { standards, ..Default::default() };
    typst_pdf::pdf(doc, &options)
}

pub struct Pdftags;

impl OutputType for Pdftags {
    type Doc = Vec<u8>;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Pdftags;

    fn is_empty(_: &Self::Doc, live: &Self::Live) -> bool {
        live.is_empty()
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        pdftags::format(doc).at(Span::detached())
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }

    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile {
        let diffs = [text_diff(a, b)];
        file_report(Self::OUTPUT, a, b, diffs)
    }
}

impl FileOutputType for Pdftags {
    fn save_ref(live: &Self::Live) -> impl AsRef<[u8]> {
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

    fn is_empty(doc: &Self::Doc, _: &Self::Live) -> bool {
        is_empty_paged_document(doc)
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        Ok(typst_svg::svg_merged(doc, Abs::pt(1.0)))
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }

    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile {
        let diffs = [image_diff(a, b, "svg+xml"), text_diff(a, b)];
        file_report(Self::OUTPUT, a, b, diffs)
    }
}

impl HashOutputType for Svg {
    const INDEX: usize = 1;
}

pub struct Html;

impl OutputType for Html {
    type Doc = HtmlDocument;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Html;

    fn is_empty(_: &Self::Doc, live: &Self::Live) -> bool {
        // HACK: This is somewhat volatile, since it needs to be updated,
        // whenever the default HTML output changes.
        const EMPTY_HTML_DOC: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
  </head>
  <body></body>
</html>
"#;
        live == EMPTY_HTML_DOC
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        typst_html::html(doc)
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }

    fn make_report(
        a: Option<(&Path, Old<&[u8]>)>,
        b: Result<(&Path, &[u8]), ()>,
    ) -> ReportFile {
        // TODO: HTML preview in iframe.
        let diffs = [text_diff(a, b)];
        file_report(Self::OUTPUT, a, b, diffs)
    }
}

impl FileOutputType for Html {
    fn save_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn matches(old: &[u8], new: &Self::Live) -> bool {
        old == new.as_bytes()
    }
}

fn image_diff(
    a: Option<(&Path, Old<&[u8]>)>,
    b: Result<(&Path, &[u8]), ()>,
    format: &str,
) -> DiffKind {
    let a = a.map(|(_, old)| old);
    let b = b.map(|(_, bytes)| bytes);
    DiffKind::Image(report::image_diff(a, b, format))
}

fn text_diff(a: Option<(&Path, Old<&[u8]>)>, b: Result<(&Path, &[u8]), ()>) -> DiffKind {
    let a = a.map(|(_, old)| old.map(|bytes| std::str::from_utf8(bytes).unwrap()));
    let b = b.map(|(_, bytes)| std::str::from_utf8(bytes).unwrap());
    DiffKind::Text(report::text_diff(a, b))
}

fn file_report(
    output: TestOutput,
    a: Option<(&Path, Old<&[u8]>)>,
    b: Result<(&Path, &[u8]), ()>,
    diffs: impl IntoIterator<Item = DiffKind>,
) -> ReportFile {
    let old = a.map(|(path, old)| File {
        path: eco_format!("{}", path.display()),
        size: old.data().map(|d| d.len()),
    });
    let new = b.ok().map(|(path, bytes)| File {
        path: eco_format!("{}", path.display()),
        size: Some(bytes.len()),
    });
    ReportFile::new(output, old, new, diffs)
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

/// Whether this content can be considered empty.
pub fn is_empty_content(content: &Content) -> bool {
    if let Some(sequence) = content.to_packed::<SequenceElem>() {
        sequence.children.iter().all(is_empty_content)
    } else {
        content.is::<SpaceElem>() || content.is::<ParbreakElem>()
    }
}

/// Whether rendering of this document can be skipped, because the only item it
/// contains are tags.
pub fn is_empty_paged_document(doc: &PagedDocument) -> bool {
    fn is_empty_frame(frame: &Frame) -> bool {
        frame.items().all(|(_, item)| match item {
            FrameItem::Group(group) => is_empty_frame(&group.frame),
            FrameItem::Tag(_) => true,
            _ => false,
        })
    }

    match doc.pages.as_slice() {
        [] => true,
        [page] => {
            page.frame.width().approx_eq(Abs::pt(120.0))
                && page.frame.height().approx_eq(Abs::pt(20.0))
                && page.fill.is_auto()
                && is_empty_frame(&page.frame)
        }
        _ => false,
    }
}
