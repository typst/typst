use crate::foundations::{Content, elem};
use crate::layout::Em;

/// Displays text in small capitals.
///
/// # Example
/// ```example
/// Hello \
/// #smallcaps[Hello]
/// ```
///
/// # Smallcaps fonts
/// By default, this uses the `smcp` and `c2sc` OpenType features on the font.
/// Not all fonts support these features. Sometimes, smallcaps are part of a
/// dedicated font. This is, for example, the case for the _Latin Modern_ family
/// of fonts. In those cases, you can use a show-set rule to customize the
/// appearance of the text in smallcaps:
///
/// ```typ
/// #show smallcaps: set text(font: "Latin Modern Roman Caps")
/// ```
///
/// When the font does not provide small capitals glyphs and `typographic` is
/// set to `{true}`, Typst will synthesize them by scaling down uppercase
/// letters. You can also force synthesis by setting `typographic` to `{false}`.
///
/// # Smallcaps headings
/// You can use a [show rule]($styling/#show-rules) to apply smallcaps
/// formatting to all your headings. In the example below, we also center-align
/// our headings and disable the standard bold font.
///
/// ```example
/// #set par(justify: true)
/// #set heading(numbering: "I.")
///
/// #show heading: smallcaps
/// #show heading: set align(center)
/// #show heading: set text(
///   weight: "regular"
/// )
///
/// = Introduction
/// #lorem(40)
/// ```
#[elem(title = "Small Capitals")]
pub struct SmallcapsElem {
    /// Whether to use small capitals glyphs from the font if available.
    ///
    /// Ideally, small capitals glyphs are provided by the font (using the
    /// `smcp` and `c2sc` OpenType features). Otherwise, Typst is able to
    /// synthesize small capitals by scaling down uppercase letters.
    ///
    /// When this is set to `{false}`, synthesized glyphs will be used
    /// regardless of whether the font provides dedicated small capitals glyphs.
    /// When `{true}`, synthesized glyphs may still be used in case the font
    /// does not provide the necessary small capitals glyphs.
    ///
    /// ```example
    /// #smallcaps(typographic: true)[Hello World] \
    /// #smallcaps(typographic: false)[Hello World]
    /// ```
    #[default(true)]
    pub typographic: bool,

    /// Whether to turn uppercase letters into small capitals as well.
    ///
    /// Unless overridden by a show rule, this enables the `c2sc` OpenType
    /// feature.
    ///
    /// ```example
    /// #smallcaps(all: true)[UNICEF] is an
    /// agency of #smallcaps(all: true)[UN].
    /// ```
    #[default(false)]
    pub all: bool,

    /// The content to display in small capitals.
    #[required]
    pub body: Content,
}

/// What becomes small capitals.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Smallcaps {
    /// Minuscules become small capitals.
    Minuscules,
    /// All letters become small capitals.
    All,
}

/// Configuration values for small capitals text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SmallcapsSettings {
    /// Whether the OpenType feature should be used if possible.
    pub typographic: bool,
    /// What becomes small capitals.
    pub sc: Smallcaps,
}

/// Default size scaling for synthesized small capitals.
pub const DEFAULT_SMALLCAPS_SIZE: Em = Em::new(0.75);
