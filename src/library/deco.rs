use super::prelude::*;
use crate::util::EcoString;

/// `strike`: Typeset striken-through text.
pub fn strike(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Strikethrough)
}

/// `underline`: Typeset underlined text.
pub fn underline(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Underline)
}

/// `overline`: Typeset text with an overline.
pub fn overline(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    line_impl(args, LineKind::Overline)
}

fn line_impl(args: &mut Args, kind: LineKind) -> TypResult<Value> {
    let stroke = args.named("stroke")?.or_else(|| args.find());
    let thickness = args.named::<Linear>("thickness")?.or_else(|| args.find());
    let offset = args.named("offset")?;
    let extent = args.named("extent")?.unwrap_or_default();
    let body: Template = args.expect("body")?;

    Ok(Value::Template(body.decorate(Decoration::Line(
        LineDecoration {
            kind,
            stroke: stroke.map(Paint::Color),
            thickness,
            offset,
            extent,
        },
    ))))
}

/// `link`: Typeset text as a link.
pub fn link(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let url = args.expect::<EcoString>("url")?;
    let body = args.find().unwrap_or_else(|| {
        let mut template = Template::new();
        template.text(url.trim_start_matches("mailto:").trim_start_matches("tel:"));
        template
    });

    Ok(Value::Template(body.decorate(Decoration::Link(url))))
}

/// A decoration for a frame.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Decoration {
    /// A link to an external resource.
    Link(EcoString),
    /// An underline/strikethrough/overline decoration.
    Line(LineDecoration),
}

impl Decoration {
    /// Apply a decoration to a child's frame.
    pub fn apply(&self, ctx: &LayoutContext, frame: &mut Frame) {
        match self {
            Decoration::Link(href) => {
                let link = Element::Link(href.to_string(), frame.size);
                frame.push(Point::zero(), link);
            }
            Decoration::Line(line) => {
                line.apply(ctx, frame);
            }
        }
    }
}

/// Defines a line that is positioned over, under or on top of text.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LineDecoration {
    /// The kind of line.
    pub kind: LineKind,
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
}

/// The kind of line decoration.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum LineKind {
    /// A line under text.
    Underline,
    /// A line through text.
    Strikethrough,
    /// A line over text.
    Overline,
}

impl LineDecoration {
    /// Apply a line decoration to a all text elements in a frame.
    pub fn apply(&self, ctx: &LayoutContext, frame: &mut Frame) {
        for i in 0 .. frame.elements.len() {
            let (pos, child) = &frame.elements[i];
            if let Element::Text(text) = child {
                let face = ctx.fonts.get(text.face_id);
                let metrics = match self.kind {
                    LineKind::Underline => face.underline,
                    LineKind::Strikethrough => face.strikethrough,
                    LineKind::Overline => face.overline,
                };

                let stroke = self.stroke.unwrap_or(text.fill);

                let thickness = self
                    .thickness
                    .map(|s| s.resolve(text.size))
                    .unwrap_or(metrics.strength.to_length(text.size));

                let offset = self
                    .offset
                    .map(|s| s.resolve(text.size))
                    .unwrap_or(-metrics.position.to_length(text.size));

                let extent = self.extent.resolve(text.size);

                let subpos = Point::new(pos.x - extent, pos.y + offset);
                let vector = Point::new(text.width + 2.0 * extent, Length::zero());
                let line = Geometry::Line(vector, thickness);

                frame.push(subpos, Element::Geometry(line, stroke));
            }
        }
    }
}
