//! Mathematical formulas.

mod frac;
mod script;

pub use frac::*;
pub use script::*;

use rex::error::{Error, LayoutError};
use rex::font::FontContext;
use rex::layout::{LayoutSettings, Style};
use rex::parser::color::RGBA;
use rex::render::{Backend, Cursor, Renderer};

use crate::font::Font;
use crate::library::layout::BlockSpacing;
use crate::library::prelude::*;
use crate::library::text::{variant, FontFamily, TextNode};

/// A piece of a mathematical formula.
#[derive(Debug, Clone, Hash)]
pub enum MathNode {
    /// Whitespace.
    Space,
    /// A forced line break.
    Linebreak,
    /// An atom in a math formula: `x`, `+`, `12`.
    Atom(EcoString),
    /// A base with optional sub and superscripts: `a_1^2`.
    Script(Arc<ScriptNode>),
    /// A fraction: `x/2`.
    Frac(Arc<FracNode>),
    /// A numbered math alignment indicator: `&`, `&&`.
    Align(usize),
    /// A row of mathematical material.
    Row(Arc<Vec<MathNode>>, Span),
}

#[node(showable)]
impl MathNode {
    /// The math font family.
    #[property(referenced)]
    pub const FAMILY: FontFamily = FontFamily::new("NewComputerModernMath");
    /// The spacing above display math.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below display math.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());

    fn construct(_: &mut Vm, _: &mut Args) -> SourceResult<Content> {
        todo!()
    }
}

impl MathNode {
    /// Whether this is a display-style node.
    pub fn display(&self) -> bool {
        match self {
            Self::Row(row, _) => {
                matches!(row.as_slice(), [MathNode::Space, .., MathNode::Space])
            }
            _ => false,
        }
    }

    /// Strip parentheses from the node.
    pub fn unparen(self) -> Self {
        if let Self::Row(row, span) = &self {
            if let [MathNode::Atom(l), .., MathNode::Atom(r)] = row.as_slice() {
                if l == "(" && r == ")" {
                    let inner = row[1 .. row.len() - 1].to_vec();
                    return Self::Row(Arc::new(inner), *span);
                }
            }
        }

        self
    }
}

impl Show for MathNode {
    fn unguard(&self, _: Selector) -> ShowNode {
        ShowNode::new(self.clone())
    }

    fn encode(&self, _: StyleChain) -> Dict {
        todo!()
    }

    fn realize(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(if self.display() {
            Content::block(
                LayoutNode::new(self.clone())
                    .aligned(Spec::with_x(Some(Align::Center.into()))),
            )
        } else {
            Content::inline(self.clone())
        })
    }

    fn finalize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
        mut realized: Content,
    ) -> SourceResult<Content> {
        let mut map = StyleMap::new();
        map.set_family(styles.get(Self::FAMILY).clone(), styles);

        if self.display() {
            realized = realized.spaced(styles.get(Self::ABOVE), styles.get(Self::BELOW));
        }

        Ok(realized.styled_with_map(map))
    }
}

impl Layout for MathNode {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        _: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let style = if self.display() { Style::Display } else { Style::Text };
        let span = match self {
            &Self::Row(_, span) => span,
            _ => Span::detached(),
        };
        Ok(vec![layout_tex(world, self, span, style, styles)?])
    }
}

/// Layout a TeX formula into a frame.
fn layout_tex(
    world: Tracked<dyn World>,
    node: &dyn Texify,
    span: Span,
    style: Style,
    styles: StyleChain,
) -> SourceResult<Frame> {
    let tex = node.texify();

    // Load the font.
    let font = world
        .book()
        .select(styles.get(MathNode::FAMILY).as_str(), variant(styles))
        .and_then(|id| world.font(id))
        .ok_or("failed to find math font")
        .at(span)?;

    // Prepare the font context.
    let ctx = font
        .math()
        .map(|math| FontContext::new(font.ttf(), math))
        .ok_or("font is not suitable for math")
        .at(span)?;

    // Layout the formula.
    let em = styles.get(TextNode::SIZE);
    let settings = LayoutSettings::new(&ctx, em.to_pt(), style);
    let renderer = Renderer::new();
    let layout = renderer
        .layout(&tex, settings)
        .map_err(|err| match err {
            Error::Parse(err) => err.to_string(),
            Error::Layout(LayoutError::Font(err)) => err.to_string(),
        })
        .at(span)?;

    // Determine the metrics.
    let (x0, y0, x1, y1) = renderer.size(&layout);
    let width = Length::pt(x1 - x0);
    let mut top = Length::pt(y1);
    let mut bottom = Length::pt(-y0);
    if style != Style::Display {
        let metrics = font.metrics();
        top = styles.get(TextNode::TOP_EDGE).resolve(styles, metrics);
        bottom = -styles.get(TextNode::BOTTOM_EDGE).resolve(styles, metrics);
    };

    // Prepare a frame rendering backend.
    let size = Size::new(width, top + bottom);
    let mut backend = FrameBackend {
        frame: {
            let mut frame = Frame::new(size);
            frame.set_baseline(top);
            frame.apply_role(Role::Formula);
            frame
        },
        baseline: top,
        font: font.clone(),
        fill: styles.get(TextNode::FILL),
        lang: styles.get(TextNode::LANG),
        colors: vec![],
    };

    // Render into the frame.
    renderer.render(&layout, &mut backend);
    Ok(backend.frame)
}

/// A ReX rendering backend that renders into a frame.
struct FrameBackend {
    frame: Frame,
    baseline: Length,
    font: Font,
    fill: Paint,
    lang: Lang,
    colors: Vec<RGBA>,
}

impl FrameBackend {
    /// The currently active fill paint.
    fn fill(&self) -> Paint {
        self.colors
            .last()
            .map(|&RGBA(r, g, b, a)| RgbaColor::new(r, g, b, a).into())
            .unwrap_or(self.fill)
    }

    /// Convert a cursor to a point.
    fn transform(&self, cursor: Cursor) -> Point {
        Point::new(Length::pt(cursor.x), self.baseline + Length::pt(cursor.y))
    }
}

impl Backend for FrameBackend {
    fn symbol(&mut self, pos: Cursor, gid: u16, scale: f64) {
        self.frame.push(
            self.transform(pos),
            Element::Text(Text {
                font: self.font.clone(),
                size: Length::pt(scale),
                fill: self.fill(),
                lang: self.lang,
                glyphs: vec![Glyph {
                    id: gid,
                    x_advance: Em::new(0.0),
                    x_offset: Em::new(0.0),
                    c: ' ',
                }],
            }),
        );
    }

    fn rule(&mut self, pos: Cursor, width: f64, height: f64) {
        self.frame.push(
            self.transform(pos),
            Element::Shape(Shape {
                geometry: Geometry::Rect(Size::new(
                    Length::pt(width),
                    Length::pt(height),
                )),
                fill: Some(self.fill()),
                stroke: None,
            }),
        );
    }

    fn begin_color(&mut self, color: RGBA) {
        self.colors.push(color);
    }

    fn end_color(&mut self) {
        self.colors.pop();
    }
}

/// Turn a math node into TeX math code.
trait Texify {
    /// Perform the conversion.
    fn texify(&self) -> EcoString;
}

impl Texify for MathNode {
    fn texify(&self) -> EcoString {
        match self {
            Self::Space => "".into(),
            Self::Linebreak => r"\\".into(),
            Self::Atom(atom) => atom.chars().map(escape_char).collect(),
            Self::Script(script) => script.texify(),
            Self::Frac(frac) => frac.texify(),
            Self::Align(_) => "".into(),
            Self::Row(row, _) => row.iter().map(Texify::texify).collect(),
        }
    }
}

#[rustfmt::skip]
fn escape_char(c: char) -> EcoString {
    match c {
        '{' | '}' | '%' | '&' | '$' | '#' => format_eco!(" \\{c} "),
        'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | 'Α' ..= 'Ω' | 'α' ..= 'ω' |
        '*' | '+' | '-' | '[' | '(' | ']' | ')' | '?' | '!' | '=' | '<' | '>' |
        ':' | ',' | ';' | '|' | '/' | '@' | '.' | '"' => c.into(),
        c => unicode_math::SYMBOLS
            .iter()
            .find(|sym| sym.codepoint == c)
            .map(|sym| format_eco!("\\{} ", sym.name))
            .unwrap_or_default(),
    }
}
