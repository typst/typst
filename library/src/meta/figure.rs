use std::str::FromStr;

use super::{Count, Counter, CounterUpdate, LocalName, Numbering, NumberingPattern};
use crate::layout::{BlockNode, VNode};
use crate::prelude::*;
use crate::text::TextNode;

/// A figure with an optional caption.
///
/// ## Example
/// ```example
/// = Pipeline
/// @fig-lab shows the central step of
/// our molecular testing pipeline.
///
/// #figure(
///   image("molecular.jpg", width: 80%),
///   caption: [
///     The molecular testing pipeline.
///   ],
/// ) <fig-lab>
/// ```
///
/// Display: Figure
/// Category: meta
#[node(Locatable, Synthesize, Count, Show, LocalName)]
pub struct FigureNode {
    /// The content of the figure. Often, an [image]($func/image).
    #[required]
    pub body: Content,

    /// The figure's caption.
    pub caption: Option<Content>,

    /// How to number the figure. Accepts a
    /// [numbering pattern or function]($func/numbering).
    #[default(Some(NumberingPattern::from_str("1").unwrap().into()))]
    pub numbering: Option<Numbering>,

    /// The vertical gap between the body and caption.
    #[default(Em::new(0.65).into())]
    pub gap: Length,
}

impl Synthesize for FigureNode {
    fn synthesize(&mut self, _: &Vt, styles: StyleChain) {
        self.push_numbering(self.numbering(styles));
    }
}

impl Show for FigureNode {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.body();

        if let Some(mut caption) = self.caption(styles) {
            if let Some(numbering) = self.numbering(styles) {
                let name = self.local_name(TextNode::lang_in(styles));
                caption = TextNode::packed(eco_format!("{name}\u{a0}"))
                    + Counter::of(Self::id())
                        .display(numbering, false)
                        .spanned(self.span())
                    + TextNode::packed(": ")
                    + caption;
            }

            realized += VNode::weak(self.gap(styles).into()).pack();
            realized += caption;
        }

        Ok(BlockNode::new()
            .with_body(Some(realized))
            .with_breakable(false)
            .pack()
            .aligned(Axes::with_x(Some(Align::Center.into()))))
    }
}

impl Count for FigureNode {
    fn update(&self) -> Option<CounterUpdate> {
        self.numbering(StyleChain::default())
            .is_some()
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for FigureNode {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Abbildung",
            Lang::ENGLISH | _ => "Figure",
        }
    }
}
