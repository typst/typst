use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Show, Smart, StyleChain, TargetElem,
};
use crate::html::{tag, HtmlElem};
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
    /// provided by the font, with a fallback to `{0.2em}` in case the font does
    /// not define the necessary metrics.
    pub baseline: Smart<Length>,

    /// The font size for synthesized subscripts.
    ///
    /// This only applies to synthesized subscripts. In other words, this has no
    /// effect if `typographic` is `{true}` and the font provides the necessary
    /// subscript glyphs.
    ///
    /// If set to `{auto}`, the size is scaled according to the metrics provided
    /// by the font, with a fallback to `{0.6em}` in case the font does not
    /// define the necessary metrics.
    pub size: Smart<TextSize>,

    /// The text to display in subscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SubElem> {
    #[typst_macros::time(name = "sub", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        if TargetElem::target_in(styles).is_html() {
            return Ok(HtmlElem::new(tag::sub)
                .with_body(Some(body))
                .pack()
                .spanned(self.span()));
        }

        show_script(
            styles,
            body,
            self.typographic(styles),
            self.baseline(styles),
            self.size(styles),
            ScriptKind::Sub,
        )
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
    /// provided by the font, with a fallback to `{-0.5em}` in case the font
    /// does not define the necessary metrics.
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
    /// by the font, with a fallback to `{0.6em}` in case the font does not
    /// define the necessary metrics.
    pub size: Smart<TextSize>,

    /// The text to display in superscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SuperElem> {
    #[typst_macros::time(name = "super", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        if TargetElem::target_in(styles).is_html() {
            return Ok(HtmlElem::new(tag::sup)
                .with_body(Some(body))
                .pack()
                .spanned(self.span()));
        }

        show_script(
            styles,
            body,
            self.typographic(styles),
            self.baseline(styles),
            self.size(styles),
            ScriptKind::Super,
        )
    }
}

fn show_script(
    styles: StyleChain,
    body: Content,
    typographic: bool,
    baseline: Smart<Length>,
    size: Smart<TextSize>,
    kind: ScriptKind,
) -> SourceResult<Content> {
    let outer_text_size = TextElem::size_in(styles);
    Ok(body.styled(TextElem::set_shift_settings(Some(ShiftSettings {
        typographic,
        shift: baseline.map(|l| -l.to_em(outer_text_size)),
        size: size.map(|t| t.0.to_em(outer_text_size)),
        kind,
    }))))
}

/// Configuration values for sub- or superscript text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ShiftSettings {
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
    pub fn default_metrics(self) -> &'static ScriptMetrics {
        match self {
            Self::Sub => &DEFAULT_SUBSCRIPT_METRICS,
            Self::Super => &DEFAULT_SUPERSCRIPT_METRICS,
        }
    }

    /// Reads the script metrics from the font table for to this script kind.
    pub fn read_metrics(self, font_metrics: &FontMetrics) -> &ScriptMetrics {
        match self {
            Self::Sub => font_metrics.subscript.as_ref(),
            Self::Super => font_metrics.superscript.as_ref(),
        }
        .unwrap_or(self.default_metrics())
    }

    /// The corresponding OpenType feature.
    pub const fn feature(self) -> Tag {
        match self {
            Self::Sub => Tag::from_bytes(b"subs"),
            Self::Super => Tag::from_bytes(b"sups"),
        }
    }
}
static DEFAULT_SUBSCRIPT_METRICS: ScriptMetrics = ScriptMetrics {
    width: Em::new(0.6),
    height: Em::new(0.6),
    horizontal_offset: Em::zero(),
    vertical_offset: Em::new(-0.2),
};

static DEFAULT_SUPERSCRIPT_METRICS: ScriptMetrics = ScriptMetrics {
    width: Em::new(0.6),
    height: Em::new(0.6),
    horizontal_offset: Em::zero(),
    vertical_offset: Em::new(0.5),
};
