//! Text decorations.

use super::prelude::*;
use super::TextNode;

/// Typeset underline, striken-through or overlined text.
#[derive(Debug, Hash)]
pub struct DecoNode<L: LineKind> {
    /// The kind of line.
    pub kind: L,
    /// The decorated contents.
    pub body: Template,
}

#[class]
impl<L: LineKind> DecoNode<L> {
    /// Stroke color of the line, defaults to the text color if `None`.
    #[shorthand]
    pub const STROKE: Option<Paint> = None;
    /// Thickness of the line's strokes (dependent on scaled font size), read
    /// from the font tables if `None`.
    #[shorthand]
    pub const THICKNESS: Option<Linear> = None;
    /// Position of the line relative to the baseline (dependent on scaled font
    /// size), read from the font tables if `None`.
    pub const OFFSET: Option<Linear> = None;
    /// Amount that the line will be longer or shorter than its associated text
    /// (dependent on scaled font size).
    pub const EXTENT: Linear = Linear::zero();
    /// Whether the line skips sections in which it would collide
    /// with the glyphs. Does not apply to strikethrough.
    pub const EVADE: bool = true;

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            kind: L::default(),
            body: args.expect::<Template>("body")?,
        }))
    }
}

impl<L: LineKind> Show for DecoNode<L> {
    fn show(&self, styles: StyleChain) -> Template {
        self.body.clone().styled(TextNode::LINES, vec![Decoration {
            line: L::LINE,
            stroke: styles.get(Self::STROKE),
            thickness: styles.get(Self::THICKNESS),
            offset: styles.get(Self::OFFSET),
            extent: styles.get(Self::EXTENT),
            evade: styles.get(Self::EVADE),
        }])
    }
}

/// Defines a line that is positioned over, under or on top of text.
///
/// For more details, see [`DecoNode`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    pub line: DecoLine,
    pub stroke: Option<Paint>,
    pub thickness: Option<Linear>,
    pub offset: Option<Linear>,
    pub extent: Linear,
    pub evade: bool,
}

/// The kind of decorative line.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DecoLine {
    /// A line under text.
    Underline,
    /// A line through text.
    Strikethrough,
    /// A line over text.
    Overline,
}

/// Different kinds of decorative lines for text.
pub trait LineKind: Debug + Default + Hash + Sync + Send + 'static {
    const LINE: DecoLine;
}

/// A line under text.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Underline;

impl LineKind for Underline {
    const LINE: DecoLine = DecoLine::Underline;
}

/// A line through text.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Strikethrough;

impl LineKind for Strikethrough {
    const LINE: DecoLine = DecoLine::Strikethrough;
}

/// A line over text.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Overline;

impl LineKind for Overline {
    const LINE: DecoLine = DecoLine::Overline;
}
