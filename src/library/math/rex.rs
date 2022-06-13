use rex::error::{Error, LayoutError};
use rex::font::FontContext;
use rex::layout::{LayoutSettings, Style};
use rex::parser::color::RGBA;
use rex::render::{Backend, Cursor, Renderer};

use crate::font::FaceId;
use crate::library::prelude::*;
use crate::library::text::{variant, FontFamily, Lang, TextNode};

/// A layout node that renders with ReX.
#[derive(Debug, Hash)]
pub struct RexNode {
    /// The TeX formula.
    pub tex: Spanned<EcoString>,
    /// Whether the formula is display-level.
    pub display: bool,
    /// The font family.
    pub family: FontFamily,
}

impl Layout for RexNode {
    fn layout(
        &self,
        ctx: &mut Context,
        _: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Frame>> {
        // Load the font.
        let span = self.tex.span;
        let face_id = ctx
            .fonts
            .select(self.family.as_str(), variant(styles))
            .ok_or("failed to find math font")
            .at(span)?;

        // Prepare the font context.
        let face = ctx.fonts.get(face_id);
        let ctx = face
            .math()
            .map(|math| FontContext::new(face.ttf(), math))
            .ok_or("font is not suitable for math")
            .at(span)?;

        // Layout the formula.
        let em = styles.get(TextNode::SIZE);
        let style = if self.display { Style::Display } else { Style::Text };
        let settings = LayoutSettings::new(&ctx, em.to_pt(), style);
        let renderer = Renderer::new();
        let layout = renderer
            .layout(&self.tex.v, settings)
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
        if !self.display {
            let metrics = face.metrics();
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
            face_id,
            fill: styles.get(TextNode::FILL),
            lang: styles.get(TextNode::LANG),
            colors: vec![],
        };

        // Render into the frame.
        renderer.render(&layout, &mut backend);

        Ok(vec![backend.frame])
    }
}

/// A ReX rendering backend that renders into a frame.
struct FrameBackend {
    frame: Frame,
    baseline: Length,
    face_id: FaceId,
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
                face_id: self.face_id,
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
