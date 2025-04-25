use comemo::Track;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    Content, Context, Func, NativeFunc, NativeFuncData, Resolve, StyleChain, cast, elem,
};
use crate::layout::{Abs, Rel};
use crate::math::{Mathy, default_lr_size};

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
    ///
    /// This value can be given as a [relative length]($relative), or a
    /// [function]($function) that receives the size of the glyph as a
    /// parameter (an absolute length) and should return a (relative) length.
    /// For example, `{x => x * 80%}` would be equivalent to just specifying `{80%}`.
    ///
    /// Note that the sizes of glyphs in math fonts come about in two ways:
    ///
    /// - First, there are pre-made variants at specific sizes. This means you
    ///   will see discrete jumps in the stretched glyph's size as you increase
    ///   the size parameter. It is up to the font how many pre-made variants
    ///   there are and what their sizes are.
    ///
    /// - Then, if the pre-made variants are all too small, a glyph of the
    ///   desired size is assembled from parts. The stretched glyph's size will
    ///   now be the exact size requested.
    ///
    /// It could be the case that only one of the above exist for a glyph in
    /// the font.
    ///
    /// The value given here is really a minimum (but if there is no assembly
    /// for the glyph, this minimum may not be reached), so the actual size of
    /// the stretched glyph may not match what you specified.
    ///
    /// ```example
    /// #for i in range(0, 15) {
    ///   $stretch(\[, size: #(10pt + i * 2pt))$
    /// }
    ///
    /// #set math.stretch(size: x => x + 0.5em)
    /// $x stretch(=)^"def" y$
    /// ```
    #[default(Rel::one().into())]
    pub size: StretchSize,
}

/// How to size a stretched glyph.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum StretchSize {
    /// Sized by the specified length.
    Rel(Rel),
    /// Resolve the size for the given base size through the specified
    /// function.
    Func(Func),
}

impl StretchSize {
    /// Resolve the stretch size given the base size.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        base: Abs,
    ) -> SourceResult<Abs> {
        Ok(match self {
            Self::Rel(rel) => *rel,
            Self::Func(func) => func
                .call(engine, Context::new(None, Some(styles)).track(), [base])?
                .cast()
                .at(func.span())?,
        }
        .resolve(styles)
        .relative_to(base))
    }

    /// Whether the size is the default used by `LrElem`.
    pub fn is_lr_default(&self) -> bool {
        *self == <default_lr_size>::data().into()
    }
}

impl From<Rel> for StretchSize {
    fn from(rel: Rel) -> Self {
        Self::Rel(rel)
    }
}

impl From<&'static NativeFuncData> for StretchSize {
    fn from(data: &'static NativeFuncData) -> Self {
        Self::Func(Func::from(data))
    }
}

cast! {
    StretchSize,
    self => match self {
        Self::Rel(v) => v.into_value(),
        Self::Func(v) => v.into_value(),
    },
    v: Rel => Self::Rel(v),
    v: Func => Self::Func(v),
}
