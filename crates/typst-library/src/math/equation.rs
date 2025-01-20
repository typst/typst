use std::num::NonZeroUsize;

use typst_utils::NonZeroExt;
use unicode_math_class::MathClass;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Show, ShowSet, Smart, StyleChain, Styles,
    Synthesize, TargetElem,
};
use crate::html::{tag, HtmlAttr, HtmlElem};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
use crate::layout::{
    AlignElem, Alignment, BlockElem, InlineElem, OuterHAlignment, SpecificAlignment,
    VAlignment,
};
use crate::math::{MathSize, MathVariant};
use crate::model::{Numbering, Outlinable, ParLine, Refable, Supplement};
use crate::text::{FontFamily, FontList, FontWeight, LocalName, TextElem};

/// A mathematical equation.
///
/// Can be displayed inline with text or as a separate block.
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

use crate::foundations::SequenceElem;
use crate::math::{AccentElem, AttachElem, FracElem, LrElem, PrimesElem};
use ecow::eco_format;
fn bla(elem: &Content, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
    dbg!(elem);
    if let Some(sequence) = elem.to_packed::<SequenceElem>() {
        let c: SourceResult<Vec<_>> =
            sequence.children.iter().map(|c| bla(c, engine, styles)).collect();
        Ok(HtmlElem::new(tag::math::mrow)
            .with_body(Some(Content::sequence(c?)))
            .pack()
            .spanned(elem.span()))
    } else if elem.to_packed::<TextElem>().is_some() {
        Ok(HtmlElem::new(tag::math::mi)
            .with_body(Some(elem.clone()))
            .pack()
            .spanned(elem.span()))
    } else if let Some(elem) = elem.to_packed::<LrElem>() {
        show_lr(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<FracElem>() {
        show_frac(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<AttachElem>() {
        show_attach(elem, engine, styles)
    } else if let Some(elem) = elem.to_packed::<AccentElem>() {
        let accent = TextElem::packed(eco_format!(" {}", elem.accent.0));
        let accent = HtmlElem::new(tag::math::mo).with_body(Some(accent)).pack();
        let body = Content::sequence([elem.base.clone(), accent]);
        Ok(HtmlElem::new(tag::math::mover)
            .with_body(Some(body))
            .pack()
            .spanned(elem.span()))
    } else {
        Ok(elem.clone())
    }
}

fn show_lr(
    elem: &Packed<LrElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    /*
    if let Some(seq) = body.to_packed::<SequenceElem>() {
        let children = &seq.children;
        match &children[..] {
            [l, mid @ .., r] => todo!(),
            _ => todo!(),
        }
    }
    dbg!(&body);
    */
    bla(&elem.body, engine, styles)
}

fn show_frac(
    elem: &Packed<FracElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let num = bla(&elem.num, engine, styles)?;
    let denom = bla(&elem.denom, engine, styles)?;
    let body = Content::sequence([num, denom]);
    Ok(HtmlElem::new(tag::math::mfrac)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}

fn show_attach(
    elem: &Packed<AttachElem>,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    let merged = elem.merge_base();
    let elem = merged.as_ref().unwrap_or(elem);

    let base = elem.base.clone();
    let sup_style_chain = styles;
    let tl = elem.tl(sup_style_chain);
    let tr = elem.tr(sup_style_chain);
    let primed = tr.as_ref().is_some_and(|content| content.is::<PrimesElem>());
    let t = elem.t(sup_style_chain);

    let sub_style_chain = styles;
    let bl = elem.bl(sub_style_chain);
    let br = elem.br(sub_style_chain);
    let b = elem.b(sub_style_chain);

    let limits = false; //base.limits().active(styles);
    let (t, tr) = match (t, tr) {
        (Some(t), Some(tr)) if primed && !limits => (None, Some(tr + t)),
        (Some(t), None) if !limits => (None, Some(t)),
        (t, tr) => (t, tr),
    };
    let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };

    let none = || HtmlElem::new(tag::math::mrow).pack();
    let br = br.map(|c| bla(&c, engine, styles)).transpose()?.unwrap_or_else(none);
    let tr = tr.map(|c| bla(&c, engine, styles)).transpose()?.unwrap_or_else(none);
    let bl = bl.map(|c| bla(&c, engine, styles)).transpose()?.unwrap_or_else(none);
    let tl = tl.map(|c| bla(&c, engine, styles)).transpose()?.unwrap_or_else(none);
    let b = b.map(|c| bla(&c, engine, styles)).transpose()?;
    let t = t.map(|c| bla(&c, engine, styles)).transpose()?;
    let prescripts = HtmlElem::new(tag::math::mprescripts).pack();

    let base = match (b, t) {
        (Some(b), Some(t)) => HtmlElem::new(tag::math::munderover)
            .with_body(Some(Content::sequence([base, b, t])))
            .pack(),
        (None, None) => base,
        _ => todo!(),
    };

    let body = Content::sequence([base, br, tr, prescripts, bl, tl]);
    Ok(HtmlElem::new(tag::math::mmultiscripts)
        .with_body(Some(body))
        .pack()
        .spanned(elem.span()))
}

impl Show for Packed<EquationElem> {
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        if TargetElem::target_in(styles).is_html() {
            let disp = if self.block(styles) { "block" } else { "inline" };
            dbg!(&self.body);
            let elem = HtmlElem::new(tag::math::math)
                .with_attr(HtmlAttr::constant("display"), disp)
                .with_body(Some(bla(&self.body, engine, styles)?));
            return Ok(elem.pack().spanned(self.span()));
        }

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
    fn outline(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<Option<Content>> {
        if !self.block(StyleChain::default()) {
            return Ok(None);
        }
        let Some(numbering) = self.numbering() else {
            return Ok(None);
        };

        // After synthesis, this should always be custom content.
        let mut supplement = match (**self).supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        };

        if !supplement.is_empty() {
            supplement += TextElem::packed("\u{a0}");
        }

        let numbers = self.counter().display_at_loc(
            engine,
            self.location().unwrap(),
            styles,
            numbering,
        )?;

        Ok(Some(supplement + numbers))
    }
}
