use std::collections::HashMap;

use ecow::eco_format;
use pdf_writer::{
    types::{ColorSpaceOperand, PaintType, TilingType},
    Filter, Name, Rect, Ref,
};

use typst::layout::{Abs, Ratio, Transform};
use typst::utils::Numeric;
use typst::visualize::{Pattern, RelativeTo};

use crate::{color::PaintEncode, resources::Remapper, Resources, WithGlobalRefs};
use crate::{content, resources::ResourcesRefs};
use crate::{transform_to_array, PdfChunk};

/// Writes the actual patterns (tiling patterns) to the PDF.
/// This is performed once after writing all pages.
pub fn write_patterns(context: &WithGlobalRefs) -> (PdfChunk, HashMap<PdfPattern, Ref>) {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
        let Some(patterns) = &resources.patterns else {
            return;
        };

        for pdf_pattern in patterns.remapper.items() {
            let PdfPattern { transform, pattern, content, .. } = pdf_pattern;
            if out.contains_key(pdf_pattern) {
                continue;
            }

            let tiling = chunk.alloc();
            out.insert(pdf_pattern.clone(), tiling);

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
            tiling_pattern.pair(Name(b"Resources"), patterns.resources.reference);

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
    });

    (chunk, out)
}

/// A pattern and its transform.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PdfPattern {
    /// The transform to apply to the pattern.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Pattern,
    /// The rendered pattern.
    pub content: Vec<u8>,
}

/// Registers a pattern with the PDF.
fn register_pattern(
    ctx: &mut content::Builder,
    pattern: &Pattern,
    on_text: bool,
    mut transforms: content::Transforms,
) -> usize {
    let patterns = ctx
        .resources
        .patterns
        .get_or_insert_with(|| Box::new(PatternRemapper::new()));

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
    let content = content::build(&mut patterns.resources, pattern.frame(), None, None);

    let pdf_pattern = PdfPattern {
        transform,
        pattern: pattern.clone(),
        content: content.content.wait().clone(),
    };

    patterns.remapper.insert(pdf_pattern)
}

impl PaintEncode for Pattern {
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_fill_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
    }

    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_stroke_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
    }
}

/// De-duplicate patterns and the resources they require to be drawn.
pub struct PatternRemapper<R> {
    /// Pattern de-duplicator.
    pub remapper: Remapper<PdfPattern>,
    /// PDF resources that are used by these patterns.
    pub resources: Resources<R>,
}

impl PatternRemapper<()> {
    pub fn new() -> Self {
        Self {
            remapper: Remapper::new("P"),
            resources: Resources::default(),
        }
    }

    /// Allocate a reference to the resource dictionary of these patterns.
    pub fn with_refs(self, refs: &ResourcesRefs) -> PatternRemapper<Ref> {
        PatternRemapper {
            remapper: self.remapper,
            resources: self.resources.with_refs(refs),
        }
    }
}
