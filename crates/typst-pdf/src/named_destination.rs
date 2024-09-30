use std::collections::{HashMap, HashSet};

use pdf_writer::writers::Destination;
use pdf_writer::{Ref, Str};
use typst::diag::SourceResult;
use typst::foundations::{Label, NativeElement};
use typst::introspection::Location;
use typst::layout::Abs;
use typst::model::HeadingElem;

use crate::{AbsExt, PdfChunk, Renumber, StrExt, WithGlobalRefs};

/// A list of destinations in the PDF document (a specific point on a specific
/// page), that have a name associated with them.
///
/// Typst creates a named destination for each heading in the document, that
/// will then be written in the document catalog. PDF readers can then display
/// them to show a clickable outline of the document.
#[derive(Default)]
pub struct NamedDestinations {
    /// A map between elements and their associated labels
    pub loc_to_dest: HashMap<Location, Label>,
    /// A sorted list of all named destinations.
    pub dests: Vec<(Label, Ref)>,
}

impl Renumber for NamedDestinations {
    fn renumber(&mut self, offset: i32) {
        for (_, reference) in &mut self.dests {
            reference.renumber(offset);
        }
    }
}

/// Fills in the map and vector for named destinations and writes the indirect
/// destination objects.
pub fn write_named_destinations(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, NamedDestinations)> {
    let mut chunk = PdfChunk::new();
    let mut out = NamedDestinations::default();
    let mut seen = HashSet::new();

    // Find all headings that have a label and are the first among other
    // headings with the same label.
    let mut matches: Vec<_> = context
        .document
        .introspector
        .query(&HeadingElem::elem().select())
        .iter()
        .filter_map(|elem| elem.location().zip(elem.label()))
        .filter(|&(_, label)| seen.insert(label))
        .collect();

    // Named destinations must be sorted by key.
    matches.sort_by_key(|&(_, label)| label);

    for (loc, label) in matches {
        // Don't encode named destinations that would exceed the limit. Those
        // will instead be encoded as normal links.
        if label.as_str().len() > Str::PDFA_LIMIT {
            continue;
        }

        let pos = context.document.introspector.position(loc);
        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        if let Some((Some(page), Some(page_ref))) =
            context.pages.get(index).zip(context.globals.pages.get(index))
        {
            let dest_ref = chunk.alloc();
            let x = pos.point.x.to_f32();
            let y = (page.content.size.y - y).to_f32();
            out.dests.push((label, dest_ref));
            out.loc_to_dest.insert(loc, label);
            chunk
                .indirect(dest_ref)
                .start::<Destination>()
                .page(*page_ref)
                .xyz(x, y, None);
        }
    }

    Ok((chunk, out))
}
