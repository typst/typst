use std::collections::HashMap;

use pdf_writer::Ref;

use crate::{AllocRefs, PdfChunk, WriteStep};

/// A PDF external graphics state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExtGState {
    // In the range 0-255, needs to be divided before being written into the graphics state!
    pub stroke_opacity: u8,
    // In the range 0-255, needs to be divided before being written into the graphics state!
    pub fill_opacity: u8,
}

impl Default for ExtGState {
    fn default() -> Self {
        Self { stroke_opacity: 255, fill_opacity: 255 }
    }
}

impl ExtGState {
    pub fn uses_opacities(&self) -> bool {
        self.stroke_opacity != 255 || self.fill_opacity != 255
    }
}

pub struct ExtGraphicsState;

impl<'a> WriteStep<AllocRefs<'a>> for ExtGraphicsState {
    type Output = HashMap<ExtGState, Ref>;

    /// Embed all used external graphics states into the PDF.
    fn run(&self, context: &AllocRefs, chunk: &mut PdfChunk, out: &mut Self::Output) {
        context.resources.write(&mut |resources| {
            for external_gs in resources.ext_gs.items() {
                if out.contains_key(external_gs) {
                    continue;
                }

                let id = chunk.alloc();
                out.insert(*external_gs, id);
                chunk
                    .ext_graphics(id)
                    .non_stroking_alpha(external_gs.fill_opacity as f32 / 255.0)
                    .stroking_alpha(external_gs.stroke_opacity as f32 / 255.0);
            }
        })
    }

    fn save(context: &mut crate::References, output: Self::Output) {
        context.ext_gs = output;
    }
}
