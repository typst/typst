use std::num::NonZeroUsize;
use std::sync::LazyLock;

use codex::styling::MathVariant;
use ecow::EcoString;
use typst_utils::NonZeroExt;
use unicode_math_class::MathClass;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Content, Label, NativeElement, Packed, ShowSet, Smart, StyleChain, Styles, Synthesize, elem,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable, Location, Tagged};
use crate::layout::{
    AlignElem, Alignment, BlockElem, OuterHAlignment, SpecificAlignment, VAlignment,
};
use crate::math::MathSize;
use crate::model::{Numbering, NumberingPattern, Outlinable, ParLine, Refable, Supplement};
use crate::text::{FontFamily, FontList, FontWeight, LocalName, Locale, TextElem};

/// A mathematical equation.
///
/// Can be displayed inline with text or as a separate block. An equation
/// becomes block-level through the presence of whitespace after the opening
/// dollar sign and whitespace before the closing dollar sign.
///
/// # Example
/// ```example
/// #set text(font: "New Computer Modern")
///
/// Let $a$, $b$, and $c$ be the side
/// lengths of a right-angled triangle.
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
/// dollar signs to create an equation. Starting and ending the equation with
/// whitespace lifts it into a separate block that is centered horizontally.
/// For more details about math syntax, see the
/// [main math page]($category/math).
#[elem(Locatable, Tagged, Synthesize, ShowSet, Count, LocalName, Refable, Outlinable)]
pub struct EquationElem {
    /// Whether the equation is displayed as a separate block.
    #[default(false)]
    pub block: bool,

    /// How to number block-level equations. Accepts a
    /// [numbering pattern or function]($numbering) taking a single number.
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

    /// An alternative description of the mathematical equation.
    ///
    /// This should describe the full equation in natural language and will be
    /// made available to Assistive Technology. You can learn more in the
    /// [Textual Representations section of the Accessibility
    /// Guide]($guides/accessibility/#textual-representations).
    ///
    /// ```example
    /// #math.equation(
    ///   alt: "integral from 1 to infinity of a x squared plus b with respect to x",
    ///   block: true,
    ///   $ integral_1^oo a x^2 + b dif x $,
    /// )
    /// ```
    pub alt: Option<EcoString>,

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
    pub variant: Option<MathVariant>,

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
    pub italic: Option<bool>,

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

    /// The locale of this element (used for the alternative description).
    #[internal]
    #[synthesized]
    pub locale: Locale,

    /// Whether to number each line of a multi-line block equation.
    ///
    /// When set to `{true}`, each line of a multi-line equation will be
    /// numbered with a sub-number like `(1a)`, `(1b)`, etc.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)", sub-number: true)
    ///
    /// $ E &= m c^2 \
    ///     &= p c + ... $
    /// ```
    #[default(false)]
    pub sub_number: bool,

    /// The numbering pattern for sub-equations.
    ///
    /// Accepts a [numbering pattern]($numbering) that can contain:
    /// - `a` for lowercase letters (a, b, c)
    /// - `A` for uppercase letters (A, B, C)
    /// - `1` for numbers (1, 2, 3)
    ///
    /// If set to `{none}`, the default pattern `(a)` is used.
    ///
    /// ```example
    /// #set math.equation(
    ///   numbering: "(1)",
    ///   sub-number: true,
    ///   sub-numbering: "(1.1)",
    /// )
    ///
    /// $ E &= m c^2 \
    ///     &= p c + ... $
    /// ```
    pub sub_numbering: Option<Numbering>,

    /// The alignment of the sub-equation numbering.
    ///
    /// By default, the alignment is `{end + horizon}`.
    #[default(SpecificAlignment::Both(OuterHAlignment::End, VAlignment::Horizon))]
    pub sub_number_align: SpecificAlignment<OuterHAlignment, VAlignment>
}

impl Synthesize for Packed<EquationElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let supplement = match self.as_ref().supplement.get_ref(styles) {
            Smart::Auto => TextElem::packed(Self::local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, styles, [self.clone().pack()])?
            }
        };

        self.supplement
            .set(Smart::Custom(Some(Supplement::Content(supplement))));

        self.locale = Some(Locale::get_in(styles));

        Ok(())
    }
}

impl ShowSet for Packed<EquationElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        if self.block.get(styles) {
            out.set(AlignElem::alignment, Alignment::CENTER);
            out.set(BlockElem::breakable, false);
            out.set(ParLine::numbering, None);
            out.set(EquationElem::size, MathSize::Display);
        } else {
            out.set(EquationElem::size, MathSize::Text);
        }
        out.set(TextElem::weight, FontWeight::from_number(450));
        out.set(
            TextElem::font,
            FontList(vec![FontFamily::new("New Computer Modern Math")]),
        );
        out
    }
}

impl Count for Packed<EquationElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (self.block.get(StyleChain::default()) && self.numbering().is_some())
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for Packed<EquationElem> {
    const KEY: &'static str = "equation";
}

impl Refable for Packed<EquationElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match self.supplement.get_cloned(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(EquationElem::ELEM)
    }

    fn numbering(&self) -> Option<&Numbering> {
        self.numbering.get_ref(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<EquationElem> {
    fn outlined(&self) -> bool {
        self.block.get(StyleChain::default()) && self.numbering().is_some()
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

/// A marker element for controlling sub-numbering of individual lines in a
/// multi-line equation.
///
/// This element can be used to mark specific lines in an equation for
/// sub-numbering or to attach a label to a specific line for referencing.
#[elem(name = "line", Locatable, Tagged, Refable, Count)]
pub struct MathLineElem {
    /// Whether this line should be numbered.
    ///
    /// - `{auto}`: Follows the global `sub-number` setting (default)
    /// - `{true}`: Force numbering for this line
    /// - `{false}`: Disable numbering for this line
    #[default(Smart::Auto)]
    pub numbered: Smart<bool>,

    /// An optional label for referencing this line.
    ///
    /// When a label is provided, the line will always be numbered regardless
    /// of the `number` setting, so that it can be referenced.
    pub line_ref: Option<EcoString>,

    /// The synthesized full number (e.g., "(1a)") for this line.
    /// This is set during layout.
    #[synthesized]
    pub number: Option<Content>,

    /// The parent equation's location.
    #[synthesized]
    pub parent_location: Option<Location>,
}

impl Count for Packed<MathLineElem> {
    fn update(&self) -> Option<CounterUpdate> {
        // Sub-equations don't update any counter
        None
    }
}

impl Refable for Packed<MathLineElem> {
    fn supplement(&self) -> Content {
        // After synthesis, use stored number or default
        TextElem::packed("Eq.")
    }

    fn counter(&self) -> Counter {
        // Use the parent equation's counter
        Counter::of(EquationElem::ELEM)
    }

    fn numbering(&self) -> Option<&Numbering> {
        // Return a simple numbering pattern for sub-lines
        // This is needed for the reference to work
        use std::str::FromStr;
        static PATTERN: LazyLock<Numbering> = LazyLock::new(|| {
            NumberingPattern::from_str("a").unwrap().into()
        });
        Some(&*PATTERN)
    }
}
