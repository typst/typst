use std::collections::{HashMap, HashSet};

use pdf_writer::{writers::Destination, Ref};
use typst::foundations::{Label, NativeElement};
use typst::introspection::Location;
use typst::layout::Abs;
use typst::model::HeadingElem;

use crate::{AbsExt, AllocRefs, PdfChunk, Renumber};

#[derive(Default)]
pub struct NamedDestinations {
    /// A map between elements and their associated labels
    pub loc_to_dest: HashMap<Location, Label>,
    /// A sorted list of all named destinations.
    pub dests: Vec<(Label, Ref)>,
}

impl Renumber for NamedDestinations {
    fn renumber(&mut self, old: Ref, new: Ref) {
        if let Some(index) = self.dests.iter().position(|x| x.1 == old) {
            self.dests[index].1 = new;
        }
    }
}

/// Fills in the map and vector for named destinations and writes the indirect
/// destination objects.
pub fn write_named_destinations(
    context: &AllocRefs,
    chunk: &mut PdfChunk,
) -> NamedDestinations {
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
        let pos = context.document.introspector.position(loc);
        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        if let Some(page) = context.resources.pages.get(index) {
            let dest_ref = chunk.alloc();
            let x = pos.point.x.to_f32();
            let y = (page.content.size.y - y).to_f32();
            out.dests.push((label, dest_ref));
            out.loc_to_dest.insert(loc, label);
            chunk
                .indirect(dest_ref)
                .start::<Destination>()
                .page(context.globals.pages[index])
                .xyz(x, y, None);
        }
    }

    out
}
