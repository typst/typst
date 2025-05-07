use std::num::NonZeroUsize;

use smallvec::{smallvec, SmallVec};
use typst_utils::{NonZeroExt, Numeric};
use unicode_math_class::MathClass;

use crate::diag::{bail, HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Array, Content, NativeElement, Packed, Resolve, Show, ShowSet, Smart,
    StyleChain, Styles, Synthesize, Value,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
use crate::layout::{
    Abs, AlignElem, Alignment, BlockElem, Fr, InlineElem, Length, OuterHAlignment, Rel,
    Spacing, SpecificAlignment, VAlignment,
};
use crate::math::{MathSize, MathVariant};
use crate::model::{Numbering, Outlinable, ParLine, Refable, Supplement};
use crate::text::{FontFamily, FontList, FontWeight, LocalName, TextElem};

/// A mathematical equation.
///
/// Can be displayed inline with text or as a separate block. An equation
/// becomes block-level through the presence of at least one space after the
/// opening dollar sign and one space before the closing dollar sign.
///
/// # Example
/// ```example
/// #set text(font: "New Computer Modern")
///
/// Let $a$, $b$, and $c$ be the side
/// lengths of right-angled triangle.
/// Then, we know that:
/// $ a^2 + b^2 = c^2 $
///
/// Prove by induction:
/// $ sum_(k=1)^n k = (n(n+1)) / 2 $
/// ```
///
/// By default, block-level equations will not break across pages. This can be
/// changed through `{show math.equation: set block(breakable: true)}`.
///
/// # Syntax
/// This function also has dedicated syntax: Write mathematical markup within
/// dollar signs to create an equation. Starting and ending the equation with at
/// least one space lifts it into a separate block that is centered
/// horizontally. For more details about math syntax, see the
/// [main math page]($category/math).
#[elem(Locatable, Synthesize, Show, ShowSet, Count, LocalName, Refable, Outlinable)]
pub struct EquationElem {
    /// Whether the equation is displayed as a separate block.
    #[default(false)]
    pub block: bool,

    /// How to [number]($numbering) block-level equations.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)")
    ///
    /// We define:
    /// $ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>
    ///
    /// With @ratio, we get:
    /// $ F_n = floor(1 / sqrt(5) phi.alt^n) $
    /// ```
    #[borrowed]
    pub numbering: Option<Numbering>,

    /// The alignment of the equation numbering.
    ///
    /// By default, the alignment is `{end + horizon}`. For the horizontal
    /// component, you can use `{right}`, `{left}`, or `{start}` and `{end}`
    /// of the text direction; for the vertical component, you can use
    /// `{top}`, `{horizon}`, or `{bottom}`.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)", number-align: bottom)
    ///
    /// We can calculate:
    /// $ E &= sqrt(m_0^2 + p^2) \
    ///     &approx 125 "GeV" $
    /// ```
    #[default(SpecificAlignment::Both(OuterHAlignment::End, VAlignment::Horizon))]
    pub number_align: SpecificAlignment<OuterHAlignment, VAlignment>,

    /// A supplement for the equation.
    ///
    /// For references to equations, this is added before the referenced number.
    ///
    /// If a function is specified, it is passed the referenced equation and
    /// should return content.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)", supplement: [Eq.])
    ///
    /// We define:
    /// $ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>
    ///
    /// With @ratio, we get:
    /// $ F_n = floor(1 / sqrt(5) phi.alt^n) $
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// The gap between columns.
    ///
    /// ```example
    /// #set math.equation(column-gap: 3em)
    /// $ 4   &= 4 & &"yes" \
    ///   0   &= 0 & &"no" \
    ///   1+1 &= 2 & &"maybe" $
    /// ```
    #[default(Fr::one().into())]
    #[borrowed]
    pub column_gap: GapSizings,

    ///
    #[default(Fr::one().into())]
    #[borrowed]
    pub column_padding: PaddingSizings,

    /// The contents of the equation.
    #[required]
    pub body: Content,

    /// The size of the glyphs.
    #[internal]
    #[default(MathSize::Text)]
    #[ghost]
    pub size: MathSize,

    /// The style variant to select.
    #[internal]
    #[ghost]
    pub variant: MathVariant,

    /// Affects the height of exponents.
    #[internal]
    #[default(false)]
    #[ghost]
    pub cramped: bool,

    /// Whether to use bold glyphs.
    #[internal]
    #[default(false)]
    #[ghost]
    pub bold: bool,

    /// Whether to use italic glyphs.
    #[internal]
    #[ghost]
    pub italic: Smart<bool>,

    /// A forced class to use for all fragment.
    #[internal]
    #[ghost]
    pub class: Option<MathClass>,

    /// Values of `scriptPercentScaleDown` and `scriptScriptPercentScaleDown`
    /// respectively in the current font's MathConstants table.
    #[internal]
    #[default((70, 50))]
    #[ghost]
    pub script_scale: (i16, i16),
}

impl Synthesize for Packed<EquationElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let supplement = match self.as_ref().supplement(styles) {
            Smart::Auto => TextElem::packed(Self::local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, styles, [self.clone().pack()])?
            }
        };

        self.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        Ok(())
    }
}

impl Show for Packed<EquationElem> {
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        if self.block(styles) {
            Ok(BlockElem::multi_layouter(
                self.clone(),
                engine.routines.layout_equation_block,
            )
            .pack()
            .spanned(self.span()))
        } else {
            Ok(InlineElem::layouter(self.clone(), engine.routines.layout_equation_inline)
                .pack()
                .spanned(self.span()))
        }
    }
}

impl ShowSet for Packed<EquationElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        if self.block(styles) {
            out.set(AlignElem::set_alignment(Alignment::CENTER));
            out.set(BlockElem::set_breakable(false));
            out.set(ParLine::set_numbering(None));
            out.set(EquationElem::set_size(MathSize::Display));
        } else {
            out.set(EquationElem::set_size(MathSize::Text));
        }
        out.set(TextElem::set_weight(FontWeight::from_number(450)));
        out.set(TextElem::set_font(FontList(vec![FontFamily::new(
            "New Computer Modern Math",
        )])));
        out
    }
}

impl Count for Packed<EquationElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (self.block(StyleChain::default()) && self.numbering().is_some())
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for Packed<EquationElem> {
    const KEY: &'static str = "equation";
}

impl Refable for Packed<EquationElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match (**self).supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(EquationElem::elem())
    }

    fn numbering(&self) -> Option<&Numbering> {
        (**self).numbering(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<EquationElem> {
    fn outlined(&self) -> bool {
        self.block(StyleChain::default()) && self.numbering().is_some()
    }

    fn prefix(&self, numbers: Content) -> Content {
        let supplement = self.supplement();
        if !supplement.is_empty() {
            supplement + TextElem::packed('\u{a0}') + numbers
        } else {
            numbers
        }
    }

    fn body(&self) -> Content {
        Content::empty()
    }
}

/// Gap sizing definitions.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GapSizings<T: Numeric = Length>(pub SmallVec<[GapSizing<T>; 1]>);

impl<T: Into<Spacing>> From<T> for GapSizings {
    fn from(spacing: T) -> Self {
        Self(smallvec![GapSizing::from(spacing)])
    }
}

impl Resolve for &GapSizings {
    type Output = GapSizings<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Self::Output {
            0: self.0.iter().map(|v| v.resolve(styles)).collect(),
        }
    }
}

cast! {
    GapSizings,
    self => self.0.into_value(),
    v: GapSizing => Self(smallvec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
}

/// Padding sizing definitions.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PaddingSizings<T: Numeric = Length>(pub SmallVec<[GapSizing<T>; 2]>);

impl<T: Into<Spacing>> From<T> for PaddingSizings {
    fn from(spacing: T) -> Self {
        let spacing = spacing.into();
        Self(smallvec![GapSizing::from(spacing), GapSizing::from(spacing)])
    }
}

impl Resolve for &PaddingSizings {
    type Output = PaddingSizings<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Self::Output {
            0: self.0.iter().map(|v| v.resolve(styles)).collect(),
        }
    }
}

cast! {
    PaddingSizings,
    self => self.0.into_value(),
    v: GapSizing => Self(smallvec![v, v]),
    v: Array => match v.as_slice() {
        [start, end] => Self(smallvec![start.clone().cast()?, end.clone().cast()?]),
        _ => bail!("expected 2 sizings, found {}", v.len()),
    },
}

/// Defines how to size a gap along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GapSizing<T: Numeric = Length> {
    /// A size specified in absolute terms and relative to the parent's size.
    Rel(Rel<T>),
    /// A size specified as a fraction of the remaining free space in the
    /// parent.
    Fr(Fr),
}

impl Resolve for GapSizing {
    type Output = GapSizing<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Self::Rel(rel) => Self::Output::Rel(rel.resolve(styles)),
            Self::Fr(fr) => Self::Output::Fr(fr),
        }
    }
}

impl<T: Into<Spacing>> From<T> for GapSizing {
    fn from(spacing: T) -> Self {
        match spacing.into() {
            Spacing::Rel(rel) => Self::Rel(rel),
            Spacing::Fr(fr) => Self::Fr(fr),
        }
    }
}

cast! {
    GapSizing,
    self => match self {
        Self::Rel(rel) => rel.into_value(),
        Self::Fr(fr) => fr.into_value(),
    },
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}
