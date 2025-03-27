use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, Smart, StyleChain};
use crate::layout::{Em, Length};
use crate::text::{variant, FontMetrics, TextElem, TextSize};
use crate::World;
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
    /// Whether to create artificial subscripts by shifting and scaling down
    /// regular glyphs.
    ///
    /// Ideally, subscripts glyphs should be provided by the font (using the
    /// `subs` OpenType feature). Otherwise, Typst is able to synthesize
    /// subscripts as explained above.
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

    /// The baseline shift for synthesized subscripts.
    ///
    /// This only applies to synthesized subscripts. In other words, this has no
    /// effect if `typographic` is `{true}` and the font provides the necessary
    /// subscript glyphs.
    pub baseline: Smart<Length>,

    /// The font size for synthesized subscripts.
    ///
    /// This only applies to synthesized subscripts. In other words, this has no
    /// effect if `typographic` is `{true}` and the font provides the necessary
    /// subscript glyphs.
    pub size: Smart<TextSize>,

    /// The text to display in subscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SubElem> {
    #[typst_macros::time(name = "sub", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        show_script(
            engine,
            styles,
            self.body.clone(),
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
    /// Whether to create artificial superscripts by shifting and scaling down
    /// regular glyphs.
    ///
    /// Ideally, superscripts glyphs should be provided by the font (using the
    /// `subs` OpenType feature). Otherwise, Typst is able to synthesize
    /// superscripts as explained above.
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

    /// The baseline shift for synthesized superscripts.
    ///
    /// This only applies to synthesized superscripts. In other words, this has
    /// no effect if `typographic` is `{true}` and the font provides the
    /// necessary superscript glyphs.
    pub baseline: Smart<Length>,

    /// The font size for synthesized superscripts.
    ///
    /// This only applies to synthesized superscripts. In other words, this has
    /// no effect if `typographic` is `{true}` and the font provides the
    /// necessary superscript glyphs.
    pub size: Smart<TextSize>,

    /// The text to display in superscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SuperElem> {
    #[typst_macros::time(name = "super", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        show_script(
            engine,
            styles,
            self.body.clone(),
            self.typographic(styles),
            self.baseline(styles),
            self.size(styles),
            ScriptKind::Super,
        )
    }
}

fn show_script(
    engine: &mut Engine,
    styles: StyleChain,
    body: Content,
    typographic: bool,
    baseline: Smart<Length>,
    size: Smart<TextSize>,
    kind: ScriptKind,
) -> SourceResult<Content> {
    let world = engine.world;
    let text = body.plain_text();
    let font = TextElem::font_in(styles).into_iter().find_map(|family| {
        let font = world
            .book()
            .select(family.as_str(), variant(styles))
            .and_then(|id| world.font(id))?;
        let covers = family.covers();
        text.chars()
            .all(|c| {
                covers.is_none_or(|cov| cov.is_match(c.encode_utf8(&mut [0; 4])))
                    && font.ttf().glyph_index(c).is_some()
            })
            .then_some(font)
    });
    let outer_text_size = TextElem::size_in(styles);
    let script_metrics =
        font.map_or(kind.default_metrics(), |f| kind.read_metrics(f.metrics()));
    Ok(body.styled(TextElem::set_subpercript(Some(ScriptSettings {
        synthesized: !typographic,
        shift: baseline
            .map_or(script_metrics.vertical_offset, |l| -l.to_em(outer_text_size)),
        size: size.map_or(script_metrics.height, |t| t.0.to_em(outer_text_size)),
        kind,
    }))))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ScriptKind {
    Sub,
    Super,
}

impl ScriptKind {
    pub fn default_metrics(self) -> ScriptMetrics {
        match self {
            Self::Sub => ScriptMetrics::default_subscript(),
            Self::Super => ScriptMetrics::default_superscript(),
        }
    }

    pub fn read_metrics(self, font_metrics: &FontMetrics) -> ScriptMetrics {
        match self {
            Self::Sub => font_metrics.subscript,
            Self::Super => font_metrics.superscript,
        }
    }

    /// The corresponding OpenType feature.
    pub fn feature(self) -> &'static [u8; 4] {
        match self {
            Self::Sub => b"subs",
            Self::Super => b"sups",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ScriptSettings {
    pub synthesized: bool,
    pub shift: Em,
    pub size: Em,
    pub kind: ScriptKind,
}
