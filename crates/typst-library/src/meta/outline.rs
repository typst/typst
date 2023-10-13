use std::str::FromStr;

use typst::util::option_eq;

use super::{
    Counter, CounterKey, HeadingElem, LocalName, Numbering, NumberingPattern, Refable,
};
use crate::layout::{BoxElem, HElem, HideElem, ParbreakElem, RepeatElem, Spacing};
use crate::prelude::*;
use crate::text::{LinebreakElem, SpaceElem, TextElem};

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
#[elem(scope, keywords = ["Table of Contents"], Show, Finalize, LocalName)]
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
    /// ```
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

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
    #[default(LocatableSelector(Selector::Elem(
        HeadingElem::elem(),
        Some(dict! { "outlined" => true })
    )))]
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
    /// *Migration hints:*  Specifying `{true}` (equivalent to `{auto}`) or
    /// `{false}` (equivalent to `{none}`) for this option is deprecated and
    /// will be removed in a future release.
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
    pub indent: Option<Smart<OutlineIndent>>,

    /// Content to fill the space between the title and the page number. Can be
    /// set to `none` to disable filling.
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

impl Show for OutlineElem {
    #[tracing::instrument(name = "OutlineElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![ParbreakElem::new().pack()];
        // Build the outline title.
        if let Some(title) = self.title(styles) {
            let title =
                title.unwrap_or_else(|| {
                    TextElem::packed(self.local_name(
                        TextElem::lang_in(styles),
                        TextElem::region_in(styles),
                    ))
                    .spanned(self.span())
                });

            seq.push(HeadingElem::new(title).with_level(NonZeroUsize::ONE).pack());
        }

        let indent = self.indent(styles);
        let depth = self.depth(styles).unwrap_or(NonZeroUsize::new(usize::MAX).unwrap());

        let mut ancestors: Vec<&Content> = vec![];
        let elems = vt.introspector.query(&self.target(styles).0);

        for elem in &elems {
            let Some(entry) = OutlineEntry::from_outlinable(
                vt,
                self.span(),
                elem.clone().into_inner(),
                self.fill(styles),
            )?
            else {
                continue;
            };

            let level = entry.level();
            if depth < level {
                continue;
            }

            // Deals with the ancestors of the current element.
            // This is only applicable for elements with a hierarchy/level.
            while ancestors
                .last()
                .and_then(|ancestor| ancestor.with::<dyn Outlinable>())
                .map_or(false, |last| last.level() >= level)
            {
                ancestors.pop();
            }

            OutlineIndent::apply(&indent, vt, &ancestors, &mut seq, self.span())?;

            // Add the overridable outline entry, followed by a line break.
            seq.push(entry.pack());
            seq.push(LinebreakElem::new().pack());

            ancestors.push(elem);
        }

        seq.push(ParbreakElem::new().pack());

        Ok(Content::sequence(seq))
    }
}

impl Finalize for OutlineElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        realized
            .styled(HeadingElem::set_outlined(false))
            .styled(HeadingElem::set_numbering(None))
    }
}

impl LocalName for OutlineElem {
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Përmbajtja",
            Lang::ARABIC => "المحتويات",
            Lang::BOKMÅL => "Innhold",
            Lang::CHINESE if option_eq(region, "TW") => "目錄",
            Lang::CHINESE => "目录",
            Lang::CZECH => "Obsah",
            Lang::DANISH => "Indhold",
            Lang::DUTCH => "Inhoudsopgave",
            Lang::FILIPINO => "Talaan ng mga Nilalaman",
            Lang::FINNISH => "Sisällys",
            Lang::FRENCH => "Table des matières",
            Lang::GERMAN => "Inhaltsverzeichnis",
            Lang::HUNGARIAN => "Tartalomjegyzék",
            Lang::ITALIAN => "Indice",
            Lang::NYNORSK => "Innhald",
            Lang::POLISH => "Spis treści",
            Lang::PORTUGUESE if option_eq(region, "PT") => "Índice",
            Lang::PORTUGUESE => "Sumário",
            Lang::ROMANIAN => "Cuprins",
            Lang::RUSSIAN => "Содержание",
            Lang::SLOVENIAN => "Kazalo",
            Lang::SPANISH => "Índice",
            Lang::SWEDISH => "Innehåll",
            Lang::TURKISH => "İçindekiler",
            Lang::UKRAINIAN => "Зміст",
            Lang::VIETNAMESE => "Mục lục",
            Lang::JAPANESE => "目次",
            Lang::ENGLISH | _ => "Contents",
        }
    }
}

/// Marks an element as being able to be outlined. This is used to implement the
/// `#outline()` element.
pub trait Outlinable: Refable {
    /// Produce an outline item for this element.
    fn outline(&self, vt: &mut Vt) -> SourceResult<Option<Content>>;

    /// Returns the nesting level of this element.
    fn level(&self) -> NonZeroUsize {
        NonZeroUsize::ONE
    }
}

#[derive(Debug, Clone)]
pub enum OutlineIndent {
    Bool(bool),
    Rel(Rel<Length>),
    Func(Func),
}

impl OutlineIndent {
    fn apply(
        indent: &Option<Smart<Self>>,
        vt: &mut Vt,
        ancestors: &Vec<&Content>,
        seq: &mut Vec<Content>,
        span: Span,
    ) -> SourceResult<()> {
        match indent {
            // 'none' | 'false' => no indenting
            None | Some(Smart::Custom(OutlineIndent::Bool(false))) => {}

            // 'auto' | 'true' => use numbering alignment for indenting
            Some(Smart::Auto | Smart::Custom(OutlineIndent::Bool(true))) => {
                // Add hidden ancestors numberings to realize the indent.
                let mut hidden = Content::empty();
                for ancestor in ancestors {
                    let ancestor_outlinable = ancestor.with::<dyn Outlinable>().unwrap();

                    if let Some(numbering) = ancestor_outlinable.numbering() {
                        let numbers = ancestor_outlinable
                            .counter()
                            .at(vt, ancestor.location().unwrap())?
                            .display(vt, &numbering)?;

                        hidden += numbers + SpaceElem::new().pack();
                    };
                }

                if !ancestors.is_empty() {
                    seq.push(HideElem::new(hidden).pack());
                    seq.push(SpaceElem::new().pack());
                }
            }

            // Length => indent with some fixed spacing per level
            Some(Smart::Custom(OutlineIndent::Rel(length))) => {
                seq.push(
                    HElem::new(Spacing::Rel(*length)).pack().repeat(ancestors.len()),
                );
            }

            // Function => call function with the current depth and take
            // the returned content
            Some(Smart::Custom(OutlineIndent::Func(func))) => {
                let depth = ancestors.len();
                let LengthOrContent(content) =
                    func.call_vt(vt, [depth])?.cast().at(span)?;
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
        Self::Bool(v) => v.into_value(),
        Self::Rel(v) => v.into_value(),
        Self::Func(v) => v.into_value()
    },
    v: bool => OutlineIndent::Bool(v),
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
    /// recommended to wrap the filling content in a [`box`]($box) with
    /// fractional width. For example, `{box(width: 1fr, repeat[-])}` would show
    /// precisely as many `-` characters as necessary to fill a particular gap.
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
        vt: &mut Vt,
        span: Span,
        elem: Content,
        fill: Option<Content>,
    ) -> SourceResult<Option<Self>> {
        let Some(outlinable) = elem.with::<dyn Outlinable>() else {
            bail!(span, "cannot outline {}", elem.func().name());
        };

        let Some(body) = outlinable.outline(vt)? else {
            return Ok(None);
        };

        let location = elem.location().unwrap();
        let page_numbering = vt
            .introspector
            .page_numbering(location)
            .cast::<Option<Numbering>>()
            .unwrap()
            .unwrap_or_else(|| {
                Numbering::Pattern(NumberingPattern::from_str("1").unwrap())
            });

        let page = Counter::new(CounterKey::Page)
            .at(vt, location)?
            .display(vt, &page_numbering)?;

        Ok(Some(Self::new(outlinable.level(), elem, body, fill, page)))
    }
}

impl Show for OutlineEntry {
    fn show(&self, _vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let mut seq = vec![];
        let elem = self.element();

        // In case a user constructs an outline entry with an arbitrary element.
        let Some(location) = elem.location() else {
            bail!(self.span(), "cannot outline {}", elem.func().name())
        };

        // The body text remains overridable.
        seq.push(self.body().linked(Destination::Location(location)));

        // Add filler symbols between the section name and page number.
        if let Some(filler) = self.fill() {
            seq.push(SpaceElem::new().pack());
            seq.push(
                BoxElem::new()
                    .with_body(Some(filler))
                    .with_width(Fr::one().into())
                    .pack(),
            );
            seq.push(SpaceElem::new().pack());
        } else {
            seq.push(HElem::new(Fr::one().into()).pack());
        }

        // Add the page number.
        let page = self.page().linked(Destination::Location(location));
        seq.push(page);

        Ok(Content::sequence(seq))
    }
}
