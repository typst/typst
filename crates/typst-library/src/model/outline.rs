use std::num::NonZeroUsize;
use std::str::FromStr;

use comemo::{Track, Tracked};
use smallvec::SmallVec;
use typst_syntax::Span;
use typst_utils::{Get, NonZeroExt};

use crate::diag::{bail, error, At, HintedStrResult, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, select_where, Args, Construct, Content, Context, Func,
    LocatableSelector, NativeElement, Packed, Resolve, Show, ShowSet, Smart, StyleChain,
    Styles,
};
use crate::introspection::{
    Counter, CounterKey, Introspector, Locatable, Location, Locator, LocatorLink,
};
use crate::layout::{
    Abs, Axes, BlockBody, BlockElem, BoxElem, Dir, Em, Fr, HElem, Length, Region, Rel,
    RepeatElem, Sides,
};
use crate::math::EquationElem;
use crate::model::{Destination, HeadingElem, NumberingPattern, ParElem, Refable};
use crate::text::{LocalName, SpaceElem, TextElem};

/// A table of contents, figures, or other elements.
///
/// This function generates a list of all occurrences of an element in the
/// document, up to a given [`depth`]($outline.depth). The element's numbering
/// and page number will be displayed in the outline alongside its title or
/// caption.
///
/// # Example
/// ```example
/// #set heading(numbering: "1.")
/// #outline()
///
/// = Introduction
/// #lorem(5)
///
/// = Methods
/// == Setup
/// #lorem(10)
/// ```
///
/// # Alternative outlines
/// In its default configuration, this function generates a table of contents.
/// By setting the `target` parameter, the outline can be used to generate a
/// list of other kinds of elements than headings.
///
/// In the example below, we list all figures containing images by setting
/// `target` to `{figure.where(kind: image)}`. Just the same, we could have set
/// it to `{figure.where(kind: table)}` to generate a list of tables.
///
/// We could also set it to just `figure`, without using a [`where`]($function.where)
/// selector, but then the list would contain _all_ figures, be it ones
/// containing images, tables, or other material.
///
/// ```example
/// #outline(
///   title: [List of Figures],
///   target: figure.where(kind: image),
/// )
///
/// #figure(
///   image("tiger.jpg"),
///   caption: [A nice figure!],
/// )
/// ```
///
/// # Styling the outline
/// At the most basic level, you can style the outline by setting properties on
/// it and its entries. This way, you can customize the outline's
/// [title]($outline.title), how outline entries are
/// [indented]($outline.indent), and how the space between an entry's text and
/// its page number should be [filled]($outline.entry.fill).
///
/// Richer customization is possible through configuration of the outline's
/// [entries]($outline.entry). The outline generates one entry for each outlined
/// element.
///
/// ## Spacing the entries { #entry-spacing }
/// Outline entries are [blocks]($block), so you can adjust the spacing between
/// them with normal block-spacing rules:
///
/// ```example
/// #show outline.entry.where(
///   level: 1
/// ): set block(above: 1.2em)
///
/// #outline()
///
/// = About ACME Corp.
/// == History
/// === Origins
/// = Products
/// == ACME Tools
/// ```
///
/// ## Building an outline entry from its parts { #building-an-entry }
/// For full control, you can also write a transformational show rule on
/// `outline.entry`. However, the logic for properly formatting and indenting
/// outline entries is quite complex and the outline entry itself only contains
/// two fields: The level and the outlined element.
///
/// For this reason, various helper functions are provided. You can mix and
/// match these to compose an entry from just the parts you like.
///
/// The default show rule for an outline entry looks like this[^1]:
/// ```typ
/// #show outline.entry: it => link(
///   it.element.location(),
///   it.indented(it.prefix(), it.inner()),
/// )
/// ```
///
/// - The [`indented`]($outline.entry.indented) function takes an optional
///   prefix and inner content and automatically applies the proper indentation
///   to it, such that different entries align nicely and long headings wrap
///   properly.
///
/// - The [`prefix`]($outline.entry.prefix) function formats the element's
///   numbering (if any). It also appends a supplement for certain elements.
///
/// - The [`inner`]($outline.entry.inner) function combines the element's
///   [`body`]($outline.entry.body), the filler, and the
///   [`page` number]($outline.entry.page).
///
/// You can use these individual functions to format the outline entry in
/// different ways. Let's say, you'd like to fully remove the filler and page
/// numbers. To achieve this, you could write a show rule like this:
///
/// ```example
/// #show outline.entry: it => link(
///   it.element.location(),
///   // Keep just the body, dropping
///   // the fill and the page.
///   it.indented(it.prefix(), it.body()),
/// )
///
/// #outline()
///
/// = About ACME Corp.
/// == History
/// ```
///
/// [^1]: The outline of equations is the exception to this rule as it does not
///       have a body and thus does not use indented layout.
#[elem(scope, keywords = ["Table of Contents", "toc"], Show, ShowSet, LocalName, Locatable)]
pub struct OutlineElem {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the
    ///   [text language]($text.lang) will be used.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    ///
    /// The outline's heading will not be numbered by default, but you can
    /// force it to be with a show-set rule:
    /// `{show outline: set heading(numbering: "1.")}`
    pub title: Smart<Option<Content>>,

    /// The type of element to include in the outline.
    ///
    /// To list figures containing a specific kind of element, like an image or
    /// a table, you can specify the desired kind in a [`where`]($function.where)
    /// selector. See the section on [alternative outlines]($outline/#alternative-outlines)
    /// for more details.
    ///
    /// ```example
    /// #outline(
    ///   title: [List of Tables],
    ///   target: figure.where(kind: table),
    /// )
    ///
    /// #figure(
    ///   table(
    ///     columns: 4,
    ///     [t], [1], [2], [3],
    ///     [y], [0.3], [0.7], [0.5],
    ///   ),
    ///   caption: [Experiment results],
    /// )
    /// ```
    #[default(LocatableSelector(HeadingElem::elem().select()))]
    #[borrowed]
    pub target: LocatableSelector,

    /// The maximum level up to which elements are included in the outline. When
    /// this argument is `{none}`, all elements are included.
    ///
    /// ```example
    /// #set heading(numbering: "1.")
    /// #outline(depth: 2)
    ///
    /// = Yes
    /// Top-level section.
    ///
    /// == Still
    /// Subsection.
    ///
    /// === Nope
    /// Not included.
    /// ```
    pub depth: Option<NonZeroUsize>,

    /// How to indent the outline's entries.
    ///
    /// - `{auto}`: Indents the numbering/prefix of a nested entry with the
    ///   title of its parent entry. If the entries are not numbered (e.g., via
    ///   [heading numbering]($heading.numbering)), this instead simply inserts
    ///   a fixed amount of `{1.2em}` indent per level.
    ///
    /// - [Relative length]($relative): Indents the entry by the specified
    ///   length per nesting level. Specifying `{2em}`, for instance, would
    ///   indent top-level headings by `{0em}` (not nested), second level
    ///   headings by `{2em}` (nested once), third-level headings by `{4em}`
    ///   (nested twice) and so on.
    ///
    /// - [Function]($function): You can further customize this setting with a
    ///   function. That function receives the nesting level as a parameter
    ///   (starting at 0 for top-level headings/elements) and should return a
    ///   (relative) length. For example, `{n => n * 2em}` would be equivalent
    ///   to just specifying `{2em}`.
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// #outline(
    ///   title: [Contents (Automatic)],
    ///   indent: auto,
    /// )
    ///
    /// #outline(
    ///   title: [Contents (Length)],
    ///   indent: 2em,
    /// )
    ///
    /// = About ACME Corp.
    /// == History
    /// === Origins
    /// #lorem(10)
    ///
    /// == Products
    /// #lorem(10)
    /// ```
    pub indent: Smart<OutlineIndent>,
}

#[scope]
impl OutlineElem {
    #[elem]
    type OutlineEntry;
}

impl Show for Packed<OutlineElem> {
    #[typst_macros::time(name = "outline", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let span = self.span();

        // Build the outline title.
        let mut seq = vec![];
        if let Some(title) = self.title(styles).unwrap_or_else(|| {
            Some(TextElem::packed(Self::local_name_in(styles)).spanned(span))
        }) {
            seq.push(
                HeadingElem::new(title)
                    .with_depth(NonZeroUsize::ONE)
                    .pack()
                    .spanned(span),
            );
        }

        let elems = engine.introspector.query(&self.target(styles).0);
        let depth = self.depth(styles).unwrap_or(NonZeroUsize::MAX);

        // Build the outline entries.
        for elem in elems {
            let Some(outlinable) = elem.with::<dyn Outlinable>() else {
                bail!(span, "cannot outline {}", elem.func().name());
            };

            let level = outlinable.level();
            if outlinable.outlined() && level <= depth {
                let entry = OutlineEntry::new(level, elem);
                seq.push(entry.pack().spanned(span));
            }
        }

        Ok(Content::sequence(seq))
    }
}

impl ShowSet for Packed<OutlineElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(HeadingElem::set_outlined(false));
        out.set(HeadingElem::set_numbering(None));
        out.set(ParElem::set_justify(false));
        out.set(BlockElem::set_above(Smart::Custom(ParElem::leading_in(styles).into())));
        // Makes the outline itself available to its entries. Should be
        // superseded by a proper ancestry mechanism in the future.
        out.set(OutlineEntry::set_parent(Some(self.clone())));
        out
    }
}

impl LocalName for Packed<OutlineElem> {
    const KEY: &'static str = "outline";
}

/// Defines how an outline is indented.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum OutlineIndent {
    /// Indents by the specified length per level.
    Rel(Rel),
    /// Resolve the indent for a specific level through the given function.
    Func(Func),
}

impl OutlineIndent {
    /// Resolve the indent for an entry with the given level.
    fn resolve(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        level: NonZeroUsize,
        span: Span,
    ) -> SourceResult<Rel> {
        let depth = level.get() - 1;
        match self {
            Self::Rel(length) => Ok(*length * depth as f64),
            Self::Func(func) => func.call(engine, context, [depth])?.cast().at(span),
        }
    }
}

cast! {
    OutlineIndent,
    self => match self {
        Self::Rel(v) => v.into_value(),
        Self::Func(v) => v.into_value()
    },
    v: Rel<Length> => Self::Rel(v),
    v: Func => Self::Func(v),
}

/// Marks an element as being able to be outlined.
pub trait Outlinable: Refable {
    /// Whether this element should be included in the outline.
    fn outlined(&self) -> bool;

    /// The nesting level of this element.
    fn level(&self) -> NonZeroUsize {
        NonZeroUsize::ONE
    }

    /// Constructs the default prefix given the formatted numbering.
    fn prefix(&self, numbers: Content, add_supplement: Smart<bool>) -> Content;

    /// The body of the entry.
    fn body(&self) -> Content;
}

/// Represents an entry line in an outline.
///
/// With show-set and show rules on outline entries, you can richly customize
/// the outline's appearance. See the
/// [section on styling the outline]($outline/#styling-the-outline) for details.
#[elem(scope, name = "entry", title = "Outline Entry", Show)]
pub struct OutlineEntry {
    /// The nesting level of this outline entry. Starts at `{1}` for top-level
    /// entries.
    #[required]
    pub level: NonZeroUsize,

    /// The element this entry refers to. Its location will be available
    /// through the [`location`]($content.location) method on the content
    /// and can be [linked]($link) to.
    #[required]
    pub element: Content,

    /// Content to fill the space between the title and the page number. Can be
    /// set to `{none}` to disable filling.
    ///
    /// The `fill` will be placed into a fractionally sized box that spans the
    /// space between the entry's body and the page number. When using show
    /// rules to override outline entries, it is thus recommended to wrap the
    /// fill in a [`box`] with fractional width, i.e.
    /// `{box(width: 1fr, it.fill)}`.
    ///
    /// When using [`repeat`], the [`gap`]($repeat.gap) property can be useful
    /// to tweak the visual weight of the fill.
    ///
    /// ```example
    /// #set outline.entry(fill: line(length: 100%))
    /// #outline()
    ///
    /// = A New Beginning
    /// ```
    #[borrowed]
    #[default(Some(
        RepeatElem::new(TextElem::packed("."))
            .with_gap(Em::new(0.15).into())
            .pack()
    ))]
    pub fill: Option<Content>,

    /// Lets outline entries access the outline they are part of. This is a bit
    /// of a hack and should be superseded by a proper ancestry mechanism.
    #[ghost]
    #[internal]
    pub parent: Option<Packed<OutlineElem>>,
}

impl Show for Packed<OutlineEntry> {
    #[typst_macros::time(name = "outline.entry", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let span = self.span();
        let context = Context::new(None, Some(styles));
        let context = context.track();

        let prefix = self.prefix(engine, context, span, Smart::Auto)?;
        let inner = self.inner(engine, context, span)?;
        let block = if self.element.is::<EquationElem>() {
            let body = prefix.unwrap_or_default() + inner;
            BlockElem::new()
                .with_body(Some(BlockBody::Content(body)))
                .pack()
                .spanned(span)
        } else {
            self.indented(engine, context, span, prefix, inner, Em::new(0.5).into())?
        };

        let loc = self.element_location().at(span)?;
        Ok(block.linked(Destination::Location(loc)))
    }
}

#[scope]
impl OutlineEntry {
    /// A helper function for producing an indented entry layout: Lays out a
    /// prefix and the rest of the entry in an indent-aware way.
    ///
    /// If the parent outline's [`indent`]($outline.indent) is `{auto}`, the
    /// inner content of all entries at level `N` is aligned with the prefix of
    /// all entries at level `N + 1`, leaving at least `gap` space between the
    /// prefix and inner parts. Furthermore, the `inner` contents of all entries
    /// at the same level are aligned.
    ///
    /// If the outline's indent is a fixed value or a function, the prefixes are
    /// indented, but the inner contents are simply inset from the prefix by the
    /// specified `gap`, rather than aligning outline-wide.
    #[func(contextual)]
    pub fn indented(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// The `prefix` is aligned with the `inner` content of entries that
        /// have level one less.
        ///
        /// In the default show rule, this is just `it.prefix()`, but it can be
        /// freely customized.
        prefix: Option<Content>,
        /// The formatted inner content of the entry.
        ///
        /// In the default show rule, this is just `it.inner()`, but it can be
        /// freely customized.
        inner: Content,
        /// The gap between the prefix and the inner content.
        #[named]
        #[default(Em::new(0.5).into())]
        gap: Length,
    ) -> SourceResult<Content> {
        let styles = context.styles().at(span)?;
        let outline = Self::parent_in(styles)
            .ok_or("must be called within the context of an outline")
            .at(span)?;
        let outline_loc = outline.location().unwrap();

        let prefix_width = prefix
            .as_ref()
            .map(|prefix| measure_prefix(engine, prefix, outline_loc, styles))
            .transpose()?;
        let prefix_inset = prefix_width.map(|w| w + gap.resolve(styles));

        let indent = outline.indent(styles);
        let (base_indent, hanging_indent) = match &indent {
            Smart::Auto => compute_auto_indents(
                engine.introspector,
                outline_loc,
                styles,
                self.level,
                prefix_inset,
            ),
            Smart::Custom(amount) => {
                let base = amount.resolve(engine, context, self.level, span)?;
                (base, prefix_inset)
            }
        };

        let body = if let (
            Some(prefix),
            Some(prefix_width),
            Some(prefix_inset),
            Some(hanging_indent),
        ) = (prefix, prefix_width, prefix_inset, hanging_indent)
        {
            // Save information about our prefix that other outline entries
            // can query for (within `compute_auto_indent`) to align
            // themselves).
            let mut seq = Vec::with_capacity(5);
            if indent.is_auto() {
                seq.push(PrefixInfo::new(outline_loc, self.level, prefix_inset).pack());
            }

            // Dedent the prefix by the amount of hanging indent and then skip
            // ahead so that the inner contents are aligned.
            seq.extend([
                HElem::new((-hanging_indent).into()).pack(),
                prefix,
                HElem::new((hanging_indent - prefix_width).into()).pack(),
                inner,
            ]);
            Content::sequence(seq)
        } else {
            inner
        };

        let inset = Sides::default().with(
            TextElem::dir_in(styles).start(),
            Some(base_indent + Rel::from(hanging_indent.unwrap_or_default())),
        );

        Ok(BlockElem::new()
            .with_inset(inset)
            .with_body(Some(BlockBody::Content(body)))
            .pack()
            .spanned(span))
    }

    /// Formats the element's numbering (if any).
    #[func(contextual)]
    pub fn prefix(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// Whether to add the element's supplement if it has one.
        ///
        /// If set to `{auto}`, the supplement is added for figures and
        /// equations that have one. For instance, it would output `[1.1]`
        /// for a heading, but `[Figure 1]` for a figure, as is usual for
        /// outlines.
        #[named]
        #[default]
        add_supplement: Smart<bool>,
    ) -> SourceResult<Option<Content>> {
        let outlinable = self.outlinable().at(span)?;
        let Some(numbering) = outlinable.numbering() else { return Ok(None) };
        let loc = self.element_location().at(span)?;
        let styles = context.styles().at(span)?;
        let numbers =
            outlinable.counter().display_at_loc(engine, loc, styles, numbering)?;
        Ok(Some(outlinable.prefix(numbers, add_supplement)))
    }

    /// Creates the default inner content of the entry.
    ///
    /// This includes the body, the fill, and page number.
    #[func(contextual)]
    pub fn inner(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<Content> {
        let styles = context.styles().at(span)?;

        let mut seq = vec![];

        // Isolate the entry body in RTL because the page number is typically
        // LTR. I'm not sure whether LTR should conceptually also be isolated,
        // but in any case we don't do it for now because the text shaping
        // pipeline does tend to choke a bit on default ignorables (in
        // particular the CJK-Latin spacing).
        //
        // See also:
        // - https://github.com/typst/typst/issues/4476
        // - https://github.com/typst/typst/issues/5176
        let rtl = TextElem::dir_in(styles) == Dir::RTL;
        if rtl {
            // "Right-to-Left Embedding"
            seq.push(TextElem::packed("\u{202B}"));
        }

        seq.push(self.body().at(span)?);

        if rtl {
            // "Pop Directional Formatting"
            seq.push(TextElem::packed("\u{202C}"));
        }

        // Add the filler between the section name and page number.
        if let Some(filler) = self.fill(styles) {
            seq.push(SpaceElem::shared().clone());
            seq.push(
                BoxElem::new()
                    .with_body(Some(filler.clone()))
                    .with_width(Fr::one().into())
                    .pack()
                    .spanned(span),
            );
            seq.push(SpaceElem::shared().clone());
        } else {
            seq.push(HElem::new(Fr::one().into()).pack().spanned(span));
        }

        // Add the page number. The word joiner in front ensures that the page
        // number doesn't stand alone in its line.
        seq.push(TextElem::packed("\u{2060}"));
        seq.push(self.page(engine, context, span)?);

        Ok(Content::sequence(seq))
    }

    /// The content which is displayed in place of the referred element at its
    /// entry in the outline. For a heading, this is its
    /// [`body`]($heading.body); for a figure a caption and for equations, it is
    /// empty.
    #[func]
    pub fn body(&self) -> StrResult<Content> {
        Ok(self.outlinable()?.body())
    }

    /// The page number of this entry's element, formatted with the numbering
    /// set for the referenced page.
    #[func(contextual)]
    pub fn page(
        &self,
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
    ) -> SourceResult<Content> {
        let loc = self.element_location().at(span)?;
        let styles = context.styles().at(span)?;
        let numbering = engine
            .introspector
            .page_numbering(loc)
            .cloned()
            .unwrap_or_else(|| NumberingPattern::from_str("1").unwrap().into());
        Counter::new(CounterKey::Page).display_at_loc(engine, loc, styles, &numbering)
    }
}

impl OutlineEntry {
    fn outlinable(&self) -> StrResult<&dyn Outlinable> {
        self.element
            .with::<dyn Outlinable>()
            .ok_or_else(|| error!("cannot outline {}", self.element.func().name()))
    }

    fn element_location(&self) -> HintedStrResult<Location> {
        let elem = &self.element;
        elem.location().ok_or_else(|| {
            if elem.can::<dyn Locatable>() && elem.can::<dyn Outlinable>() {
                error!(
                    "{} must have a location", elem.func().name();
                    hint: "try using a show rule to customize the outline.entry instead",
                )
            } else {
                error!("cannot outline {}", elem.func().name())
            }
        })
    }
}

cast! {
    OutlineEntry,
    v: Content => v.unpack::<Self>().map_err(|_| "expected outline entry")?
}

/// Measures the width of a prefix.
fn measure_prefix(
    engine: &mut Engine,
    prefix: &Content,
    loc: Location,
    styles: StyleChain,
) -> SourceResult<Abs> {
    let pod = Region::new(Axes::splat(Abs::inf()), Axes::splat(false));
    let link = LocatorLink::measure(loc);
    Ok((engine.routines.layout_frame)(engine, prefix, Locator::link(&link), styles, pod)?
        .width())
}

/// Compute the base indent and hanging indent for an auto-indented outline
/// entry of the given level, with the given prefix inset.
fn compute_auto_indents(
    introspector: Tracked<Introspector>,
    outline_loc: Location,
    styles: StyleChain,
    level: NonZeroUsize,
    prefix_inset: Option<Abs>,
) -> (Rel, Option<Abs>) {
    let indents = query_prefix_widths(introspector, outline_loc);

    let fallback = Em::new(1.2).resolve(styles);
    let get = |i: usize| indents.get(i).copied().flatten().unwrap_or(fallback);

    let last = level.get() - 1;
    let base: Abs = (0..last).map(get).sum();
    let hang = prefix_inset.map(|p| p.max(get(last)));

    (base.into(), hang)
}

/// Determines the maximum prefix inset (prefix width + gap) at each outline
/// level, for the outline with the given `loc`. Levels for which there is no
/// information available yield `None`.
#[comemo::memoize]
fn query_prefix_widths(
    introspector: Tracked<Introspector>,
    outline_loc: Location,
) -> SmallVec<[Option<Abs>; 4]> {
    let mut widths = SmallVec::<[Option<Abs>; 4]>::new();
    let elems = introspector.query(&select_where!(PrefixInfo, Key => outline_loc));
    for elem in &elems {
        let info = elem.to_packed::<PrefixInfo>().unwrap();
        let level = info.level.get();
        if widths.len() < level {
            widths.resize(level, None);
        }
        widths[level - 1].get_or_insert(info.inset).set_max(info.inset);
    }
    widths
}

/// Helper type for introspection-based prefix alignment.
#[elem(Construct, Locatable, Show)]
struct PrefixInfo {
    /// The location of the outline this prefix is part of. This is used to
    /// scope prefix computations to a specific outline.
    #[required]
    key: Location,

    /// The level of this prefix's entry.
    #[required]
    #[internal]
    level: NonZeroUsize,

    /// The width of the prefix, including the gap.
    #[required]
    #[internal]
    inset: Abs,
}

impl Construct for PrefixInfo {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Show for Packed<PrefixInfo> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
