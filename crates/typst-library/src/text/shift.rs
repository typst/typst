use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, Smart, StyleChain};
use crate::layout::{Em, Length};
use crate::text::{FontMetrics, TextElem, TextSize};
use ttf_parser::Tag;
use typst_library::text::ScriptMetrics;

/// Renders text in subscript.
///
/// The text is rendered smaller and its baseline is lowered.
///
/// # Example
/// ```example
/// Revenue#sub[yearly]
/// ```
#[elem(title = "Subscript", Show)]
pub struct SubElem {
    /// Whether to create artificial subscripts by lowering and scaling down
    /// regular glyphs.
    ///
    /// Ideally, subscripts glyphs are provided by the font (using the `subs`
    /// OpenType feature). Otherwise, Typst is able to synthesize subscripts.
    ///
    /// When this is set to `{false}`, synthesized glyphs will be used
    /// regardless of whether the font provides dedicated subscript glyphs. When
    /// `{true}`, synthesized glyphs may still be used in case the font does not
    /// provide the necessary subscript glyphs.
    ///
    /// ```example
    /// N#sub(typographic: true)[1]
    /// N#sub(typographic: false)[1]
    /// ```
    #[default(true)]
    pub typographic: bool,

    /// The downward baseline shift for synthesized subscripts.
    ///
    /// This only applies to synthesized subscripts. In other words, this has no
    /// effect if `typographic` is `{true}` and the font provides the necessary
    /// subscript glyphs.
    ///
    /// If set to `{auto}`, the baseline is shifted according to the metrics
    /// provided by the font.
    pub baseline: Smart<Length>,

    /// The font size for synthesized subscripts.
    ///
    /// This only applies to synthesized subscripts. In other words, this has no
    /// effect if `typographic` is `{true}` and the font provides the necessary
    /// subscript glyphs.
    ///
    /// If set to `{auto}`, the size is scaled according to the metrics provided
    /// by the font.
    pub size: Smart<TextSize>,

    /// The text to display in subscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SubElem> {
    #[typst_macros::time(name = "sub", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let outer_text_size = TextElem::size_in(styles);
        Ok(self
            .body
            .clone()
            .styled(TextElem::set_subperscript(Some(ScriptSettings {
                typographic: self.typographic(styles),
                shift: self.baseline(styles).map(|l| -l.to_em(outer_text_size)),
                size: self.size(styles).map(|t| t.0.to_em(outer_text_size)),
                kind: ScriptKind::Sub,
            }))))
    }
}

/// Renders text in superscript.
///
/// The text is rendered smaller and its baseline is raised.
///
/// # Example
/// ```example
/// 1#super[st] try!
/// ```
#[elem(title = "Superscript", Show)]
pub struct SuperElem {
    /// Whether to create artificial superscripts by raising and scaling down
    /// regular glyphs.
    ///
    /// Ideally, superscripts glyphs are provided by the font (using the `sups`
    /// OpenType feature). Otherwise, Typst is able to synthesize superscripts.
    ///
    /// When this is set to `{false}`, synthesized glyphs will be used
    /// regardless of whether the font provides dedicated superscript glyphs.
    /// When `{true}`, synthesized glyphs may still be used in case the font
    /// does not provide the necessary superscript glyphs.
    ///
    /// ```example
    /// N#super(typographic: true)[1]
    /// N#super(typographic: false)[1]
    /// ```
    #[default(true)]
    pub typographic: bool,

    /// The downward baseline shift for synthesized superscripts.
    ///
    /// This only applies to synthesized superscripts. In other words, this has
    /// no effect if `typographic` is `{true}` and the font provides the
    /// necessary superscript glyphs.
    ///
    /// If set to `{auto}`, the baseline is shifted according to the metrics
    /// provided by the font.
    ///
    /// Note that, since the baseline shift is applied downward, you will need
    /// to provide a negative value for the content to appear as raised above
    /// the normal baseline.
    pub baseline: Smart<Length>,

    /// The font size for synthesized superscripts.
    ///
    /// This only applies to synthesized superscripts. In other words, this has
    /// no effect if `typographic` is `{true}` and the font provides the
    /// necessary superscript glyphs.
    ///
    /// If set to `{auto}`, the size is scaled according to the metrics provided
    /// by the font.
    pub size: Smart<TextSize>,

    /// The text to display in superscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SuperElem> {
    #[typst_macros::time(name = "super", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let outer_text_size = TextElem::size_in(styles);
        Ok(self
            .body
            .clone()
            .styled(TextElem::set_subperscript(Some(ScriptSettings {
                typographic: self.typographic(styles),
                shift: self.baseline(styles).map(|l| -l.to_em(outer_text_size)),
                size: self.size(styles).map(|t| t.0.to_em(outer_text_size)),
                kind: ScriptKind::Super,
            }))))
    }
}

/// Configuration values for sub- or superscript text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ScriptSettings {
    /// Whether the OpenType feature should be used if possible.
    pub typographic: bool,
    /// The baseline shift of the script, relative to the outer text size.
    ///
    /// For superscripts, this is positive. For subscripts, this is negative. A
    /// value of [`Smart::Auto`] indicates that the value should be obtained
    /// from font metrics.
    pub shift: Smart<Em>,
    /// The size of the script, relative to the outer text size.
    ///
    /// A value of [`Smart::Auto`] indicates that the value should be obtained
    /// from font metrics.
    pub size: Smart<Em>,
    /// The kind of script (either a subscript, or a superscript).
    ///
    /// This is used to know which OpenType table to use to resolve
    /// [`Smart::Auto`] values.
    pub kind: ScriptKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ScriptKind {
    Sub,
    Super,
}

impl ScriptKind {
    /// Returns the default metrics for this script kind.
    ///
    /// This can be used as a last resort if neither the user nor the font
    /// provided those metrics.
    pub const fn default_metrics(self) -> ScriptMetrics {
        match self {
            Self::Sub => ScriptMetrics::default_subscript(),
            Self::Super => ScriptMetrics::default_superscript(),
        }
    }

    /// Reads the script metrics from the font table for to this script kind.
    pub const fn read_metrics(self, font_metrics: &FontMetrics) -> ScriptMetrics {
        match self {
            Self::Sub => font_metrics.subscript,
            Self::Super => font_metrics.superscript,
        }
    }

    /// The corresponding OpenType feature.
    pub const fn feature(self) -> Tag {
        match self {
            Self::Sub => Tag::from_bytes(b"subs"),
            Self::Super => Tag::from_bytes(b"sups"),
        }
    }
}
