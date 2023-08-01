use super::*;
use crate::meta::Supplement;
use crate::meta::{Counter, Numbering, Outlinable, Refable};
use crate::prelude::*;
use crate::text::TextElem;

/// FIXME Documentation
/// Display: Equation Label
/// Category: Math
#[element(LayoutMath)]
pub struct MathLabelElem {
    #[required]
    pub value: EcoString,
}

impl LayoutMath for MathLabelElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.push(MathFragment::Linebreak(MathLabel::Some(Label(self.value()))));
        Ok(())
    }
}

/// FIXME Documentation
/// Display: Equation Label
/// Category: Math
#[element(LayoutMath)]
pub struct NoNumberElem {}
impl LayoutMath for NoNumberElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.push(MathFragment::Linebreak(MathLabel::NoNumber));
        Ok(())
    }
}

/// FIXME: Documentation
/// This element represents an equation number. It's main purpose
/// in life is to keep the equation number counter, and for labels
/// to have something to attach themselves to.
/// Display: Equation Number
/// Category: Math
#[element(Count, Locatable, Outlinable, Refable, Show)]
pub struct EqNumberElem {
    pub numbering: Option<Numbering>,

    pub supplement: Option<Supplement>,
}

impl Show for EqNumberElem {
    // #[tracing::instrument(name = "UpdateElem::show", skip(self))]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

impl Outlinable for EqNumberElem {
    fn outline(&self, vt: &mut Vt) -> SourceResult<Option<Content>> {
        let Some(numbering) = self.numbering(StyleChain::default()) else {
            return Ok(None);
        };

        let mut supplement = match self.supplement(StyleChain::default()) {
            Some(Supplement::Content(content)) => content,
            _ => Content::empty(),
        };

        if !supplement.is_empty() {
            supplement += TextElem::packed("\u{a0}");
        }

        let numbers = self
            .counter()
            .at(vt, self.0.location().unwrap())?
            .display(vt, &numbering)?;

        Ok(Some(supplement + numbers))
    }
}

impl Refable for EqNumberElem {
    fn supplement(&self) -> Content {
        match self.supplement(StyleChain::default()) {
            Some(Supplement::Content(content)) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(Self::func())
    }

    fn numbering(&self) -> Option<Numbering> {
        self.numbering(StyleChain::default())
    }
}

impl Count for EqNumberElem {
    fn update(&self) -> Option<CounterUpdate> {
        // `EquationElem::layout` handles updating this counter.
        None
    }
}
