use typst::font::FontWeight;

use super::{LocalName, Numbering};
use crate::layout::{BlockNode, HNode, VNode};
use crate::prelude::*;
use crate::text::{TextNode, TextSize};

/// A section heading.
///
/// With headings, you can structure your document into sections. Each heading
/// has a _level,_ which starts at one and is unbounded upwards. This level
/// indicates the logical role of the following content (section, subsection,
/// etc.)  A top-level heading indicates a top-level section of the document
/// (not the document's title).
///
/// Typst can automatically number your headings for you. To enable numbering,
/// specify how you want your headings to be numbered with a
/// [numbering pattern or function]($func/numbering).
///
/// Independently from the numbering, Typst can also automatically generate an
/// [outline]($func/outline) of all headings for you. To exclude one or more
/// headings from this outline, you can set the `outlined` parameter to
/// `{false}`.
///
/// ## Example
/// ```example
/// #set heading(numbering: "1.a)")
///
/// = Introduction
/// In recent years, ...
///
/// == Preliminaries
/// To start, ...
/// ```
///
/// ## Syntax
/// Headings have dedicated syntax: They can be created by starting a line with
/// one or multiple equals signs, followed by a space. The number of equals
/// signs determines the heading's logical nesting depth.
///
/// Display: Heading
/// Category: meta
#[node(Locatable, Synthesize, Show, Finalize, LocalName)]
pub struct HeadingNode {
    /// The logical nesting depth of the heading, starting from one.
    #[default(NonZeroUsize::new(1).unwrap())]
    pub level: NonZeroUsize,

    /// How to number the heading. Accepts a
    /// [numbering pattern or function]($func/numbering).
    ///
    /// ```example
    /// #set heading(numbering: "1.a.")
    ///
    /// = A section
    /// == A subsection
    /// === A sub-subsection
    /// ```
    pub numbering: Option<Numbering>,

    /// Whether the heading should appear in the outline.
    ///
    /// ```example
    /// #outline()
    ///
    /// #heading[Normal]
    /// This is a normal heading.
    ///
    /// #heading(outlined: false)[Hidden]
    /// This heading does not appear
    /// in the outline.
    /// ```
    #[default(true)]
    pub outlined: bool,

    /// The heading's title.
    #[required]
    pub body: Content,

    /// The heading's numbering numbers.
    #[synthesized]
    pub numbers: Option<Vec<NonZeroUsize>>,
}

impl Synthesize for HeadingNode {
    fn synthesize(&mut self, vt: &Vt, styles: StyleChain) {
        let my_id = self.0.stable_id();
        let numbering = self.numbering(styles);

        let mut counter = HeadingCounter::new();
        if numbering.is_some() {
            // Advance past existing headings.
            for heading in vt
                .locate_node::<Self>()
                .take_while(|figure| figure.0.stable_id() != my_id)
            {
                if heading.numbering(StyleChain::default()).is_some() {
                    counter.advance(heading);
                }
            }

            // Advance passed self.
            counter.advance(self);
        }

        self.push_level(self.level(styles));
        self.push_outlined(self.outlined(styles));
        self.push_numbers(numbering.is_some().then(|| counter.take()));
        self.push_numbering(numbering);
    }
}

impl Show for HeadingNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();
        if let Some(numbering) = self.numbering(styles) {
            let numbers = self.numbers().unwrap();
            realized = numbering.apply(vt.world(), &numbers)?.display()
                + HNode::new(Em::new(0.3).into()).with_weak(true).pack()
                + realized;
        }
        Ok(BlockNode::new().with_body(Some(realized)).pack())
    }
}

impl Finalize for HeadingNode {
    fn finalize(&self, realized: Content, styles: StyleChain) -> Content {
        let level = self.level(styles).get();
        let scale = match level {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };

        let size = Em::new(scale);
        let above = Em::new(if level == 1 { 1.8 } else { 1.44 }) / scale;
        let below = Em::new(0.75) / scale;

        let mut map = StyleMap::new();
        map.set(TextNode::set_size(TextSize(size.into())));
        map.set(TextNode::set_weight(FontWeight::BOLD));
        map.set(BlockNode::set_above(VNode::block_around(above.into())));
        map.set(BlockNode::set_below(VNode::block_around(below.into())));
        map.set(BlockNode::set_sticky(true));
        realized.styled_with_map(map)
    }
}

/// Counts through headings with different levels.
pub struct HeadingCounter(Vec<NonZeroUsize>);

impl HeadingCounter {
    /// Create a new heading counter.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Advance the counter and return the numbers for the given heading.
    pub fn advance(&mut self, heading: &HeadingNode) -> &[NonZeroUsize] {
        let level = heading.level(StyleChain::default()).get();

        if self.0.len() >= level {
            self.0[level - 1] = self.0[level - 1].saturating_add(1);
            self.0.truncate(level);
        }

        while self.0.len() < level {
            self.0.push(NonZeroUsize::new(1).unwrap());
        }

        &self.0
    }

    /// Take out the current counts.
    pub fn take(self) -> Vec<NonZeroUsize> {
        self.0
    }
}

cast_from_value! {
    HeadingNode,
    v: Content => v.to::<Self>().ok_or("expected heading")?.clone(),
}

impl LocalName for HeadingNode {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Abschnitt",
            Lang::ENGLISH | _ => "Section",
        }
    }
}
