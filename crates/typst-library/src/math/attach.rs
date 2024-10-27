use crate::foundations::{elem, Content, Packed, Smart};
use crate::layout::{Length, Rel};
use crate::math::{EquationElem, Mathy};

/// A base with optional attachments.
///
/// ```example
/// $ attach(
///   Pi, t: alpha, b: beta,
///   tl: 1, tr: 2+3, bl: 4+5, br: 6,
/// ) $
/// ```
#[elem(Mathy)]
pub struct AttachElem {
    /// The base to which things are attached.
    #[required]
    pub base: Content,

    /// The top attachment, smartly positioned at top-right or above the base.
    ///
    /// You can wrap the base in `{limits()}` or `{scripts()}` to override the
    /// smart positioning.
    pub t: Option<Content>,

    /// The bottom attachment, smartly positioned at the bottom-right or below
    /// the base.
    ///
    /// You can wrap the base in `{limits()}` or `{scripts()}` to override the
    /// smart positioning.
    pub b: Option<Content>,

    /// The top-left attachment (before the base).
    pub tl: Option<Content>,

    /// The bottom-left attachment (before base).
    pub bl: Option<Content>,

    /// The top-right attachment (after the base).
    pub tr: Option<Content>,

    /// The bottom-right attachment (after the base).
    pub br: Option<Content>,
}

impl Packed<AttachElem> {
    /// If an AttachElem's base is also an AttachElem, merge attachments into the
    /// base AttachElem where possible.
    pub fn merge_base(&self) -> Option<Self> {
        // Extract from an EquationElem.
        let mut base = self.base();
        if let Some(equation) = base.to_packed::<EquationElem>() {
            base = equation.body();
        }

        // Move attachments from elem into base where possible.
        if let Some(base) = base.to_packed::<AttachElem>() {
            let mut elem = self.clone();
            let mut base = base.clone();

            macro_rules! merge {
                ($content:ident) => {
                    if base.$content.is_none() && elem.$content.is_some() {
                        base.$content = elem.$content.clone();
                        elem.$content = None;
                    }
                };
            }

            merge!(t);
            merge!(b);
            merge!(tl);
            merge!(tr);
            merge!(bl);
            merge!(br);

            elem.base = base.pack();
            return Some(elem);
        }

        None
    }
}

/// Grouped primes.
///
/// ```example
/// $ a'''_b = a^'''_b $
/// ```
///
/// # Syntax
/// This function has dedicated syntax: use apostrophes instead of primes. They
/// will automatically attach to the previous element, moving superscripts to
/// the next level.
#[elem(Mathy)]
pub struct PrimesElem {
    /// The number of grouped primes.
    #[required]
    pub count: usize,
}

/// Forces a base to display attachments as scripts.
///
/// ```example
/// $ scripts(sum)_1^2 != sum_1^2 $
/// ```
#[elem(Mathy)]
pub struct ScriptsElem {
    /// The base to attach the scripts to.
    #[required]
    pub body: Content,
}

/// Forces a base to display attachments as limits.
///
/// ```example
/// $ limits(A)_1^2 != A_1^2 $
/// ```
#[elem(Mathy)]
pub struct LimitsElem {
    /// The base to attach the limits to.
    #[required]
    pub body: Content,

    /// Whether to also force limits in inline equations.
    ///
    /// When applying limits globally (e.g., through a show rule), it is
    /// typically a good idea to disable this.
    #[default(true)]
    pub inline: bool,
}

/// Stretches a glyph.
///
/// This function can also be used to automatically stretch the base of an
/// attachment, so that it fits the top and bottom attachments.
///
/// Note that only some glyphs can be stretched, and which ones can depend on
/// the math font being used. However, most math fonts are the same in this
/// regard.
///
/// ```example
/// $ H stretch(=)^"define" U + p V $
/// $ f : X stretch(->>, size: #150%)_"surjective" Y $
/// $ x stretch(harpoons.ltrb, size: #3em) y
///     stretch(\[, size: #150%) z $
/// ```
#[elem(Mathy)]
pub struct StretchElem {
    /// The glyph to stretch.
    #[required]
    pub body: Content,

    /// The size to stretch to, relative to the maximum size of the glyph and
    /// its attachments.
    pub size: Smart<Rel<Length>>,
}
