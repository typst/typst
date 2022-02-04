//! Text decorations.

use super::prelude::*;
use super::TextNode;

/// Typeset underline, striken-through or overlined text.
pub struct DecoNode<L: LineKind>(pub L);

#[class]
impl<L: LineKind> DecoNode<L> {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let deco = Decoration {
            line: L::LINE,
            stroke: args.named("stroke")?.or_else(|| args.find()),
            thickness: args.named::<Linear>("thickness")?.or_else(|| args.find()),
            offset: args.named("offset")?,
            extent: args.named("extent")?.unwrap_or_default(),
            evade: args.named("evade")?.unwrap_or(true),
        };
        Ok(args.expect::<Node>("body")?.styled(TextNode::LINES, vec![deco]))
    }
}

/// Defines a line that is positioned over, under or on top of text.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    /// Which line to draw.
    pub line: DecoLine,
    /// Stroke color of the line, defaults to the text color if `None`.
    pub stroke: Option<Paint>,
    /// Thickness of the line's strokes (dependent on scaled font size), read
    /// from the font tables if `None`.
    pub thickness: Option<Linear>,
    /// Position of the line relative to the baseline (dependent on scaled font
    /// size), read from the font tables if `None`.
    pub offset: Option<Linear>,
    /// Amount that the line will be longer or shorter than its associated text
    /// (dependent on scaled font size).
    pub extent: Linear,
    /// Whether the line skips sections in which it would collide
    /// with the glyphs. Does not apply to strikethrough.
    pub evade: bool,
}

impl From<DecoLine> for Decoration {
    fn from(line: DecoLine) -> Self {
        Self {
            line,
            stroke: None,
            thickness: None,
            offset: None,
            extent: Linear::zero(),
            evade: true,
        }
    }
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
pub trait LineKind {
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
