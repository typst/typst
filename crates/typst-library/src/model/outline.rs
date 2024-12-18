use std::num::NonZeroUsize;
use std::str::FromStr;

use comemo::Track;
use typst_syntax::Span;
use typst_utils::NonZeroExt;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, select_where, Content, Context, Func, LocatableSelector,
    NativeElement, Packed, Show, ShowSet, Smart, StyleChain, Styles,
};
use crate::introspection::{Counter, CounterKey, Locatable};
use crate::layout::{
    BoxElem, Dir, Em, Fr, HElem, HideElem, Length, Rel, RepeatElem, Spacing,
};
use crate::model::{
    Destination, HeadingElem, NumberingPattern, ParElem, ParbreakElem, Refable,
};
use crate::text::{LinebreakElem, LocalName, SpaceElem, TextElem};

/// A table of contents, figures, or other elements.
///
/// This function generates a list of all occurrences of an element in the
/// document, up to a given depth. The element's numbering and page number will
/// be displayed in the outline alongside its title or caption. By default this
/// generates a table of contents.
///
/// # Example
/// ```example
/// #outline()
///
/// = Introduction
/// #lorem(5)
///
/// = Prior work
/// #lorem(10)
/// ```
///
/// # Alternative outlines
/// By setting the `target` parameter, the outline can be used to generate a
/// list of other kinds of elements than headings. In the example below, we list
/// all figures containing images by setting `target` to `{figure.where(kind:
/// image)}`. We could have also set it to just `figure`, but then the list
/// would also include figures containing tables or other material. For more
/// details on the `where` selector, [see here]($function.where).
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
/// The outline element has several options for customization, such as its
/// `title` and `indent` parameters. If desired, however, it is possible to have
/// more control over the outline's look and style through the
/// [`outline.entry`]($outline.entry) element.
#[elem(scope, keywords = ["Table of Contents"], Show, ShowSet, LocalName)]
pub struct OutlineElem {
    /// The title of the outline.
    ///
    /// - When set to `{auto}`, an appropriate title for the
    ///   [text language]($text.lang) will be used. This is the default.
    /// - When set to `{none}`, the outline will not have a title.
    /// - A custom title can be set by passing content.
    ///
    /// The outline's heading will not be numbered by default, but you can
    /// force it to be with a show-set rule:
    /// `{show outline: set heading(numbering: "1.")}`
    pub title: Smart<Option<Content>>,

    /// The type of element to include in the outline.
    ///
    /// To list figures containing a specific kind of element, like a table, you
    /// can write `{figure.where(kind: table)}`.
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
    #[default(LocatableSelector(select_where!(HeadingElem, Outlined => true)))]
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
    /// - `{none}`: No indent
    /// - `{auto}`: Indents the numbering of the nested entry with the title of
    ///   its parent entry. This only has an effect if the entries are numbered
    ///   (e.g., via [heading numbering]($heading.numbering)).
    /// - [Relative length]($relative): Indents the item by this length
    ///   multiplied by its nesting level. Specifying `{2em}`, for instance,
    ///   would indent top-level headings (not nested) by `{0em}`, second level
    ///   headings by `{2em}` (nested once), third-level headings by `{4em}`
    ///   (nested twice) and so on.
    /// - [Function]($function): You can completely customize this setting with
    ///   a function. That function receives the nesting level as a parameter
    ///   (starting at 0 for top-level headings/elements) and can return a
    ///   relative length or content making up the indent. For example,
    ///   `{n => n * 2em}` would be equivalent to just specifying `{2em}`, while
    ///   `{n => [→ ] * n}` would indent with one arrow per nesting level.
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
    /// #outline(
    ///   title: [Contents (Function)],
    ///   indent: n => [→ ] * n,
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
    #[default(None)]
    #[borrowed]
    pub indent: Option<Smart<OutlineIndent>>,

    /// Content to fill the space between the title and the page number. Can be
    /// set to `{none}` to disable filling.
    ///
    /// ```example
    /// #outline(fill: line(length: 100%))
    ///
    /// = A New Beginning
    /// ```
    #[default(Some(RepeatElem::new(TextElem::packed(".")).pack()))]
    pub fill: Option<Content>,
}

#[scope]
impl OutlineElem {
    #[elem]
    type OutlineEntry;
}

impl Show for Packed<OutlineElem> {
    #[typst_macros::time(name = "outline", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![ParbreakElem::shared().clone()];
        // Build the outline title.
        if let Some(title) = self.title(styles).unwrap_or_else(|| {
            Some(TextElem::packed(Self::local_name_in(styles)).spanned(self.span()))
        }) {
            seq.push(
                HeadingElem::new(title)
                    .with_depth(NonZeroUsize::ONE)
                    .pack()
                    .spanned(self.span()),
            );
        }

        let indent = self.indent(styles);
        let depth = self.depth(styles).unwrap_or(NonZeroUsize::new(usize::MAX).unwrap());

        let mut ancestors: Vec<&Content> = vec![];
        let elems = engine.introspector.query(&self.target(styles).0);

        for elem in &elems {
            let Some(entry) = OutlineEntry::from_outlinable(
                engine,
                self.span(),
                elem.clone(),
                self.fill(styles),
                styles,
            )?
            else {
                continue;
            };

            let level = entry.level();
            if depth < *level {
                continue;
            }

            // Deals with the ancestors of the current element.
            // This is only applicable for elements with a hierarchy/level.
            while ancestors
                .last()
                .and_then(|ancestor| ancestor.with::<dyn Outlinable>())
                .is_some_and(|last| last.level() >= *level)
            {
                ancestors.pop();
            }

            OutlineIndent::apply(
                indent,
                engine,
                &ancestors,
                &mut seq,
                styles,
                self.span(),
            )?;

            // Add the overridable outline entry, followed by a line break.
            seq.push(entry.pack().spanned(self.span()));
            seq.push(LinebreakElem::shared().clone());

            ancestors.push(elem);
        }

        seq.push(ParbreakElem::shared().clone());

        Ok(Content::sequence(seq))
    }
}

impl ShowSet for Packed<OutlineElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(HeadingElem::set_outlined(false));
        out.set(HeadingElem::set_numbering(None));
        out.set(ParElem::set_first_line_indent(Em::new(0.0).into()));
        out
    }
}

impl LocalName for Packed<OutlineElem> {
    const KEY: &'static str = "outline";
}

/// Marks an element as being able to be outlined. This is used to implement the
/// `#outline()` element.
pub trait Outlinable: Refable {
    /// Produce an outline item for this element.
    fn outline(
        &self,
        engine: &mut Engine,

        styles: StyleChain,
    ) -> SourceResult<Option<Content>>;

    /// Returns the nesting level of this element.
    fn level(&self) -> NonZeroUsize {
        NonZeroUsize::ONE
    }
}

/// Defines how an outline is indented.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum OutlineIndent {
    Rel(Rel<Length>),
    Func(Func),
}

impl OutlineIndent {
    fn apply(
        indent: &Option<Smart<Self>>,
        engine: &mut Engine,
        ancestors: &Vec<&Content>,
        seq: &mut Vec<Content>,
        styles: StyleChain,
        span: Span,
    ) -> SourceResult<()> {
        match indent {
            // 'none' | 'false' => no indenting
            None => {}

            // 'auto' | 'true' => use numbering alignment for indenting
            Some(Smart::Auto) => {
                // Add hidden ancestors numberings to realize the indent.
                let mut hidden = Content::empty();
                for ancestor in ancestors {
                    let ancestor_outlinable = ancestor.with::<dyn Outlinable>().unwrap();

                    if let Some(numbering) = ancestor_outlinable.numbering() {
                        let numbers = ancestor_outlinable.counter().display_at_loc(
                            engine,
                            ancestor.location().unwrap(),
                            styles,
                            numbering,
                        )?;

                        hidden += numbers + SpaceElem::shared().clone();
                    };
                }

                if !ancestors.is_empty() {
                    seq.push(HideElem::new(hidden).pack().spanned(span));
                    seq.push(SpaceElem::shared().clone().spanned(span));
                }
            }

            // Length => indent with some fixed spacing per level
            Some(Smart::Custom(OutlineIndent::Rel(length))) => {
                seq.push(
                    HElem::new(Spacing::Rel(*length))
                        .pack()
                        .spanned(span)
                        .repeat(ancestors.len()),
                );
            }

            // Function => call function with the current depth and take
            // the returned content
            Some(Smart::Custom(OutlineIndent::Func(func))) => {
                let depth = ancestors.len();
                let LengthOrContent(content) = func
                    .call(engine, Context::new(None, Some(styles)).track(), [depth])?
                    .cast()
                    .at(span)?;
                if !content.is_empty() {
                    seq.push(content);
                }
            }
        };

        Ok(())
    }
}

cast! {
    OutlineIndent,
    self => match self {
        Self::Rel(v) => v.into_value(),
        Self::Func(v) => v.into_value()
    },
    v: Rel<Length> => OutlineIndent::Rel(v),
    v: Func => OutlineIndent::Func(v),
}

struct LengthOrContent(Content);

cast! {
    LengthOrContent,
    v: Rel<Length> => Self(HElem::new(Spacing::Rel(v)).pack()),
    v: Content => Self(v),
}

/// Represents each entry line in an outline, including the reference to the
/// outlined element, its page number, and the filler content between both.
///
/// This element is intended for use with show rules to control the appearance
/// of outlines. To customize an entry's line, you can build it from scratch by
/// accessing the `level`, `element`, `body`, `fill` and `page` fields on the
/// entry.
///
/// ```example
/// #set heading(numbering: "1.")
///
/// #show outline.entry.where(
///   level: 1
/// ): it => {
///   v(12pt, weak: true)
///   strong(it)
/// }
///
/// #outline(indent: auto)
///
/// = Introduction
/// = Background
/// == History
/// == State of the Art
/// = Analysis
/// == Setup
/// ```
#[elem(name = "entry", title = "Outline Entry", Show)]
pub struct OutlineEntry {
    /// The nesting level of this outline entry. Starts at `{1}` for top-level
    /// entries.
    #[required]
    pub level: NonZeroUsize,

    /// The element this entry refers to. Its location will be available
    /// through the [`location`]($content.location) method on content
    /// and can be [linked]($link) to.
    #[required]
    pub element: Content,

    /// The content which is displayed in place of the referred element at its
    /// entry in the outline. For a heading, this would be its number followed
    /// by the heading's title, for example.
    #[required]
    pub body: Content,

    /// The content used to fill the space between the element's outline and
    /// its page number, as defined by the outline element this entry is
    /// located in. When `{none}`, empty space is inserted in that gap instead.
    ///
    /// Note that, when using show rules to override outline entries, it is
    /// recommended to wrap the filling content in a [`box`] with fractional
    /// width. For example, `{box(width: 1fr, repeat[-])}` would show precisely
    /// as many `-` characters as necessary to fill a particular gap.
    #[required]
    pub fill: Option<Content>,

    /// The page number of the element this entry links to, formatted with the
    /// numbering set for the referenced page.
    #[required]
    pub page: Content,
}

impl OutlineEntry {
    /// Generates an OutlineEntry from the given element, if possible (errors if
    /// the element does not implement `Outlinable`). If the element should not
    /// be outlined (e.g. heading with 'outlined: false'), does not generate an
    /// entry instance (returns `Ok(None)`).
    fn from_outlinable(
        engine: &mut Engine,
        span: Span,
        elem: Content,
        fill: Option<Content>,
        styles: StyleChain,
    ) -> SourceResult<Option<Self>> {
        let Some(outlinable) = elem.with::<dyn Outlinable>() else {
            bail!(span, "cannot outline {}", elem.func().name());
        };

        let Some(body) = outlinable.outline(engine, styles)? else {
            return Ok(None);
        };

        let location = elem.location().unwrap();
        let page_numbering = engine
            .introspector
            .page_numbering(location)
            .cloned()
            .unwrap_or_else(|| NumberingPattern::from_str("1").unwrap().into());

        let page = Counter::new(CounterKey::Page).display_at_loc(
            engine,
            location,
            styles,
            &page_numbering,
        )?;

        Ok(Some(Self::new(outlinable.level(), elem, body, fill, page)))
    }
}

impl Show for Packed<OutlineEntry> {
    #[typst_macros::time(name = "outline.entry", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![];
        let elem = self.element();

        // In case a user constructs an outline entry with an arbitrary element.
        let Some(location) = elem.location() else {
            if elem.can::<dyn Locatable>() && elem.can::<dyn Outlinable>() {
                bail!(
                    self.span(), "{} must have a location", elem.func().name();
                    hint: "try using a query or a show rule to customize the outline.entry instead",
                )
            } else {
                bail!(self.span(), "cannot outline {}", elem.func().name())
            }
        };

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

        seq.push(self.body().clone().linked(Destination::Location(location)));

        if rtl {
            // "Pop Directional Formatting"
            seq.push(TextElem::packed("\u{202C}"));
        }

        // Add filler symbols between the section name and page number.
        if let Some(filler) = self.fill() {
            seq.push(SpaceElem::shared().clone());
            seq.push(
                BoxElem::new()
                    .with_body(Some(filler.clone()))
                    .with_width(Fr::one().into())
                    .pack()
                    .spanned(self.span()),
            );
            seq.push(SpaceElem::shared().clone());
        } else {
            seq.push(HElem::new(Fr::one().into()).pack().spanned(self.span()));
        }

        // Add the page number.
        let page = self.page().clone().linked(Destination::Location(location));
        seq.push(page);

        Ok(Content::sequence(seq))
    }
}
