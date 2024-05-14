use ecow::eco_format;
use pdf_writer::{
    types::{ColorSpaceOperand, PaintType, TilingType},
    Filter, Name, Rect, Ref,
};

use typst::layout::{Abs, Ratio, Transform};
use typst::util::Numeric;
use typst::visualize::{Pattern, RelativeTo};

use crate::content::{self, Resource, ResourceKind};
use crate::{color::PaintEncode, color_font::MaybeColorFont};
use crate::{transform_to_array, PdfChunk, PdfContext, PdfResource, Renumber};

pub struct Patterns;

impl PdfResource for Patterns {
    type Output = Vec<WrittenPattern>;

    /// Writes the actual patterns (tiling patterns) to the PDF.
    /// This is performed once after writing all pages.
    fn write(&self, context: &PdfContext, chunk: &mut PdfChunk) -> Self::Output {
        let pattern_map = &context.remapped_patterns;
        let mut patterns = Vec::new();

        for PdfPattern { transform, pattern, content, .. } in pattern_map {
            let tiling = chunk.alloc();
            let resources = chunk.alloc();
            patterns
                .push(WrittenPattern { pattern_ref: tiling, resources_ref: resources });

            let mut tiling_pattern = chunk.tiling_pattern(tiling, content);
            tiling_pattern
                .tiling_type(TilingType::ConstantSpacing)
                .paint_type(PaintType::Colored)
                .bbox(Rect::new(
                    0.0,
                    0.0,
                    pattern.size().x.to_pt() as _,
                    pattern.size().y.to_pt() as _,
                ))
                .x_step((pattern.size().x + pattern.spacing().x).to_pt() as _)
                .y_step((pattern.size().y + pattern.spacing().y).to_pt() as _);

            // The actual resource dict will be written in a later step
            tiling_pattern.pair(Name(b"Resources"), resources);

            tiling_pattern
                .matrix(transform_to_array(
                    transform
                        .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()))
                        .post_concat(Transform::translate(
                            Abs::zero(),
                            pattern.spacing().y,
                        )),
                ))
                .filter(Filter::FlateDecode);
        }

        patterns
    }

    fn save(context: &mut crate::References, output: Self::Output) {
        context.patterns = output;
    }
}

pub struct WrittenPattern {
    /// Reference to the pattern itself
    pub pattern_ref: Ref,
    /// Reference to the resources dictionnary this pattern uses
    pub resources_ref: Ref,
}

impl Renumber for WrittenPattern {
    fn renumber(&mut self, old: Ref, new: Ref) {
        self.pattern_ref.renumber(old, new);
        self.resources_ref.renumber(old, new);
    }
}

/// A pattern and its transform.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PdfPattern<R> {
    /// The transform to apply to the pattern.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Pattern,
    /// The rendered pattern.
    pub content: Vec<u8>,
    /// The resources used by the pattern.
    pub resources: Vec<(Resource, R)>,
}

/// Registers a pattern with the PDF.
fn register_pattern<C: MaybeColorFont>(
    ctx: &mut content::Builder<C>,
    pattern: &Pattern,
    on_text: bool,
    mut transforms: content::Transforms,
) -> usize {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let transform = match pattern.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.transform,
        RelativeTo::Parent => transforms.container_transform,
    };

    // Render the body.
    let content = content::build(ctx.parent, pattern.frame());

    let mut pdf_pattern = PdfPattern {
        transform,
        pattern: pattern.clone(),
        content: content.content.wait().clone(),
        resources: content.resources.into_iter().collect(),
    };

    pdf_pattern.resources.sort();

    ctx.parent.patterns.insert(pdf_pattern)
}

impl PaintEncode for Pattern {
    fn set_as_fill<C: MaybeColorFont>(
        &self,
        ctx: &mut content::Builder<C>,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_fill_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
        ctx.resources.insert(Resource::new(ResourceKind::Pattern, id), index);
    }

    fn set_as_stroke<C: MaybeColorFont>(
        &self,
        ctx: &mut content::Builder<C>,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_stroke_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
        ctx.resources.insert(Resource::new(ResourceKind::Pattern, id), index);
    }
}
