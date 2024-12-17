use std::collections::HashMap;

use ecow::eco_format;
use pdf_writer::types::{ColorSpaceOperand, PaintType, TilingType};
use pdf_writer::{Filter, Name, Rect, Ref};
use typst_library::diag::SourceResult;
use typst_library::layout::{Abs, Ratio, Transform};
use typst_library::visualize::{RelativeTo, Tiling};
use typst_utils::Numeric;

use crate::color::PaintEncode;
use crate::resources::{Remapper, ResourcesRefs};
use crate::{content, transform_to_array, PdfChunk, Resources, WithGlobalRefs};

/// Writes the actual patterns (tiling patterns) to the PDF.
/// This is performed once after writing all pages.
pub fn write_tilings(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<PdfTiling, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
        let Some(patterns) = &resources.tilings else {
            return Ok(());
        };

        for pdf_pattern in patterns.remapper.items() {
            let PdfTiling { transform, pattern, content, .. } = pdf_pattern;
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

        Ok(())
    })?;

    Ok((chunk, out))
}

/// A pattern and its transform.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PdfTiling {
    /// The transform to apply to the pattern.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Tiling,
    /// The rendered pattern.
    pub content: Vec<u8>,
}

/// Registers a pattern with the PDF.
fn register_pattern(
    ctx: &mut content::Builder,
    pattern: &Tiling,
    on_text: bool,
    mut transforms: content::Transforms,
) -> SourceResult<usize> {
    let patterns = ctx
        .resources
        .tilings
        .get_or_insert_with(|| Box::new(TilingRemapper::new()));

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
    let content = content::build(
        ctx.options,
        &mut patterns.resources,
        pattern.frame(),
        None,
        None,
    )?;

    let pdf_pattern = PdfTiling {
        transform,
        pattern: pattern.clone(),
        content: content.content.wait().clone(),
    };

    Ok(patterns.remapper.insert(pdf_pattern))
}

impl PaintEncode for Tiling {
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()> {
        ctx.reset_fill_color_space();

        let index = register_pattern(ctx, self, on_text, transforms)?;
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
        Ok(())
    }

    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()> {
        ctx.reset_stroke_color_space();

        let index = register_pattern(ctx, self, on_text, transforms)?;
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
        Ok(())
    }
}

/// De-duplicate patterns and the resources they require to be drawn.
pub struct TilingRemapper<R> {
    /// Pattern de-duplicator.
    pub remapper: Remapper<PdfTiling>,
    /// PDF resources that are used by these patterns.
    pub resources: Resources<R>,
}

impl TilingRemapper<()> {
    pub fn new() -> Self {
        Self {
            remapper: Remapper::new("P"),
            resources: Resources::default(),
        }
    }

    /// Allocate a reference to the resource dictionary of these patterns.
    pub fn with_refs(self, refs: &ResourcesRefs) -> TilingRemapper<Ref> {
        TilingRemapper {
            remapper: self.remapper,
            resources: self.resources.with_refs(refs),
        }
    }
}
