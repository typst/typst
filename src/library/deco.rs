//! Text decorations.

use super::prelude::*;
use super::TextNode;

/// Typeset underline, striken-through or overlined text.
#[derive(Debug, Hash)]
pub struct DecoNode<const L: DecoLine>(pub Template);

#[class]
impl<const L: DecoLine> DecoNode<L> {
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
        Ok(Template::show(Self(args.expect::<Template>("body")?)))
    }
}

impl<const L: DecoLine> Show for DecoNode<L> {
    fn show(&self, styles: StyleChain) -> Template {
        self.0.clone().styled(TextNode::LINES, vec![Decoration {
            line: L,
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

/// A kind of decorative line.
pub type DecoLine = usize;

/// A line under text.
pub const UNDERLINE: DecoLine = 0;

/// A line through text.
pub const STRIKETHROUGH: DecoLine = 1;

/// A line over text.
pub const OVERLINE: DecoLine = 2;
