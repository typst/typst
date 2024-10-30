use std::collections::HashMap;

use pdf_writer::Ref;
use typst_library::diag::SourceResult;

use crate::{PdfChunk, WithGlobalRefs};

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

/// Embed all used external graphics states into the PDF.
pub fn write_graphic_states(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<ExtGState, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
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

        Ok(())
    })?;

    Ok((chunk, out))
}
