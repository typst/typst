//! Cooperates with `docs/components/example.typ`.

use std::sync::LazyLock;

use comemo::Tracked;
use either::Either;
use typst::diag::{At, FileError, FileResult, SourceResult, Trace, Tracepoint, bail};
use typst::engine::Engine;
use typst::foundations::{
    Args, Array, Bytes, Construct, Content, Context, Datetime, Derived, Duration,
    NativeElement, Packed, Resolve, ShowFn, Smart, StyleChain, Target, TargetElem, array,
    cast, elem, func,
};
use typst::layout::{
    Abs, BlockElem, Frame, FrameItem, Margin, PageElem, Point, Ratio, Rel, Size,
    Transform,
};
use typst::loading::{DataSource, LoadSource, Loaded};
use typst::syntax::{FileId, RangeMapper, Source, Span, Spanned, VirtualRoot};
use typst::text::{Font, FontBook, RawContent, RawElem};
use typst::visualize::{
    Color, Curve, ImageElem, ImageFormat, PixelEncoding, PixelFormat, RasterFormat,
};
use typst::{Features, Library, LibraryExt, World, WorldExt};
use typst_layout::{Page, PagedDocument};
use typst_utils::LazyHash;

/// Processes a code example in the docs and returns an array of `image`
/// elements with the resulting rendered pages.
///
/// Handles `<<<` and `>>>` markers:
/// - The `<<<` marker indicates that a line is only shown in the source and not
///   actually compiled.
/// - The `>>>` marker indicates that a line is compiled, but not shown in the
///   source.
///
/// This function handles the compilation side of it. The preview side is
/// separate handled by the Typst code that renders the example's sources.
#[func]
pub fn compile_example(
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    /// The `raw` element that defines the example. The full element is passed
    /// instead of a string to retain span information, which allows diagnostics
    /// to point _into_ the example in case of an error.
    raw: Packed<RawElem>,
    /// Whether to trim all but the first page.
    #[named]
    #[default(false)]
    single: bool,
    /// If given, should be an array of four relative lengths which define a
    /// `(x, y, w, h)` trimmed view into the first page. Providing this implies
    /// `single: true`.
    #[named]
    #[default]
    zoom: Option<Zoom>,
    /// Whether warnings from the example should be propagated.
    #[named]
    #[default(true)]
    warnings: bool,
) -> SourceResult<Vec<Content>> {
    let styles = context.styles().at(span)?;
    let source = create_source(&raw, engine.world)?;
    let world = ExampleWorld(source);
    let warned = typst::compile::<PagedDocument>(&world);

    if warnings {
        for warning in warned.warnings {
            engine.sink.warn(warning);
        }
    }

    let tracepoint = || Tracepoint::Call(None);
    let mut pages = warned
        .output
        .trace(engine.world, tracepoint, raw.span())?
        .pages()
        .to_vec();

    if single || zoom.is_some() {
        pages.truncate(1);
        if let Some(zoom) = zoom {
            trim_page(&mut pages[0], &zoom, styles);
        }
    }

    let convert = match styles.get(TargetElem::target) {
        Target::Paged => page_to_frame,
        _ => page_to_image,
    };

    Ok(pages.into_iter().map(convert).collect())
}

/// Creates a `Source` containing the example's contents from an example code
/// block.
///
/// The nodes in the resulting `Source` contain span information that is mapped
/// from the example code block such that errors that occur during compilation
/// of the example are attributed to the real source.
fn create_source(
    raw: &Packed<RawElem>,
    world: Tracked<dyn World + '_>,
) -> SourceResult<Source> {
    let file_id = raw
        .span()
        .id()
        .ok_or("cannot hygienically compile example without span")
        .at(raw.span())?;

    let lines = match &raw.text {
        RawContent::Text(text) => {
            Either::Left(text.lines().map(|line| (line, Span::detached())))
        }
        RawContent::Lines(lines) => {
            Either::Right(lines.iter().map(|(line, span)| (line.as_str(), *span)))
        }
    };

    let mut compile = String::new();
    let mut ranges = Vec::new();
    for (line, span) in lines {
        if line.starts_with("<<< ") {
            continue;
        }

        let line = line.strip_prefix(">>>").unwrap_or(line);
        compile.push_str(line);
        compile.push('\n');

        if let Some(range) = world.range(span) {
            let start = range.start;
            let end = start + line.len() + 1;
            ranges.push(start..end);
        }
    }

    let mapper = RangeMapper::new(ranges);
    let mut root = typst::syntax::parse(&compile);
    root.synthesize_with(|range| match mapper.map(range) {
        Some(mapped) => Span::from_range(file_id, mapped),
        None => Span::detached(),
    });

    Ok(Source::with_root(file_id, compile, root))
}

/// Applies a zoom trim to a page.
fn trim_page(page: &mut Page, Zoom { x, y, w, h }: &Zoom, styles: StyleChain) {
    let size = page.frame.size();
    page.frame.translate(Point::new(
        -x.resolve(styles).relative_to(size.x),
        -y.resolve(styles).relative_to(size.y),
    ));
    page.frame.set_size(Size::new(
        w.resolve(styles).relative_to(size.x),
        h.resolve(styles).relative_to(size.y),
    ));
}

/// Turns a compiled `Page` into a Typst `image` element by rendering it.
fn page_to_image(page: Page) -> Content {
    let pixmap = typst_render::render(&page, 2.0);
    let format = ImageFormat::Raster(RasterFormat::Pixel(PixelFormat {
        encoding: PixelEncoding::Rgba8,
        width: pixmap.width(),
        height: pixmap.height(),
    }));
    let data = Bytes::new(pixmap.take());
    let source = DataSource::Bytes(data.clone());
    let derived = Loaded::new(Spanned::new(LoadSource::Bytes, Span::detached()), data);
    ImageElem::new(Derived { source, derived })
        .with_format(Smart::Custom(format))
        .pack()
}

/// Turns a compiled `Page` into a `FrameElem` which natively embeds it.
fn page_to_frame(mut page: Page) -> Content {
    page.frame.fill(Color::WHITE);
    drop_tags(&mut page.frame);
    FrameElem::new(page.frame).pack()
}

fn drop_tags(frame: &mut Frame) {
    frame.retain(|item| match item {
        FrameItem::Group(group) => {
            group.parent = None;
            group.label = None;
            drop_tags(&mut group.frame);
            !group.frame.is_empty()
        }
        FrameItem::Tag(_) => false,
        FrameItem::Link(..) => false,
        _ => true,
    });
}

#[elem(Construct)]
pub struct FrameElem {
    #[required]
    #[internal]
    frame: Frame,
}

impl Construct for FrameElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

pub const FRAME_RULE: ShowFn<FrameElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), |elem, _, _, _, region| {
        let mut frame = elem.frame.clone();
        let scale = (region.size.x / frame.width())
            .min(region.size.y / frame.height())
            .min(1.0);
        if scale != 1.0 {
            let s = Ratio::new(scale);
            frame.transform(Transform::scale(s, s));
            frame.set_size(frame.size() * scale);
        }
        frame.clip(Curve::rect(frame.size()));
        Ok(frame)
    })
    .pack())
};

/// Defines a trimmed view into a rendered page of an example.
pub struct Zoom {
    x: Rel,
    y: Rel,
    w: Rel,
    h: Rel,
}

cast! {
    Zoom,
    self => array![self.x, self.y, self.w, self.h].into_value(),
    v: Array => match v.as_slice() {
        [x, y, w, h] => Zoom {
            x: x.clone().cast()?,
            y: y.clone().cast()?,
            w: w.clone().cast()?,
            h: h.clone().cast()?,
        },
        _ => bail!("expected four components"),
    }
}

/// The world for example compilations.
struct ExampleWorld(Source);

impl World for ExampleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &EXAMPLE_LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &crate::world::FONTS.0
    }

    fn main(&self) -> FileId {
        self.0.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.0.id() {
            Ok(self.0.clone())
        } else {
            // Examples don't support imports.
            Err(FileError::NotFound(id.vpath().get_without_slash().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.0.id() {
            return Ok(Bytes::from_string(self.0.clone()));
        }

        // Relative file loads of assets are allowed from anywhere without full
        // paths. E.g. `json("monday.json")` from `loading/json.rs`.
        if let VirtualRoot::Project = id.root()
            && let Some(name) = id.vpath().file_name()
            && let Some(asset) = typst_dev_assets::get_by_name(name)
        {
            return Ok(Bytes::new(asset));
        }

        Err(FileError::NotFound(id.vpath().get_without_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        crate::world::FONTS.1.get(index).cloned()
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}

static EXAMPLE_LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| {
    let mut lib = Library::builder().with_features(Features::all()).build();

    // Adjust the default look a bit.
    lib.styles.set(PageElem::width, Smart::Custom(Abs::pt(240.0).into()));
    lib.styles.set(PageElem::height, Smart::Auto);
    lib.styles
        .set(PageElem::margin, Margin::splat(Some(Smart::Custom(Abs::pt(15.0).into()))));

    LazyHash::new(lib)
});
