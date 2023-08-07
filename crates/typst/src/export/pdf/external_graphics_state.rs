use crate::export::pdf::{PdfContext, RefExt};
use pdf_writer::Finish;

/// A PDF external graphics state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExternalGraphicsState {
    // In the range 0-255, needs to be divided before being written into the graphics state!
    pub stroke_opacity: u8,
    // In the range 0-255, needs to be divided before being written into the graphics state!
    pub fill_opacity: u8,
}

impl Default for ExternalGraphicsState {
    fn default() -> Self {
        Self { stroke_opacity: 255, fill_opacity: 255 }
    }
}

impl ExternalGraphicsState {
    pub fn uses_opacities(&self) -> bool {
        self.stroke_opacity != 255 || self.fill_opacity != 255
    }
}

/// Embed all used external graphics states into the PDF.
#[tracing::instrument(skip_all)]
pub fn write_external_graphics_states(ctx: &mut PdfContext) {
    for external_gs in ctx.ext_gs_map.items() {
        let gs_ref = ctx.alloc.bump();
        ctx.ext_gs_refs.push(gs_ref);

        let mut gs = ctx.writer.ext_graphics(gs_ref);
        gs.non_stroking_alpha(external_gs.fill_opacity as f32 / 255.0)
            .stroking_alpha(external_gs.stroke_opacity as f32 / 255.0);
        gs.finish();
    }
}
