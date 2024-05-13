use std::collections::{HashMap, HashSet};

use pdf_writer::{writers::Destination, Ref};
use typst::foundations::{Label, NativeElement};
use typst::introspection::Location;
use typst::layout::Abs;
use typst::model::HeadingElem;

use crate::{AbsExt, ConstructContext, PdfChunk, PdfResource, Renumber, WriteContext};

pub struct NamedDestinations;

pub struct NamedDestinationsOutput {
    dests: Vec<(Label, Ref)>,
    loc_to_dest: HashMap<Location, Label>,
}

impl Renumber for NamedDestinationsOutput {
    fn renumber(&mut self, old: Ref, new: Ref) {
        if let Some(index) = self.dests.iter().position(|x| x.1 == old) {
            self.dests[index].1 = new;
        }
    }
}

impl PdfResource for NamedDestinations {
    type Output = NamedDestinationsOutput;

    /// Fills in the map and vector for named destinations and writes the indirect
    /// destination objects.
    fn write(&self, context: &ConstructContext, chunk: &mut PdfChunk) -> Self::Output {
        let mut seen = HashSet::new();
        let mut loc_to_dest = HashMap::new();
        let mut dests = Vec::new();

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

            if let Some(page) = context.pages.get(index) {
                let dest_ref = chunk.alloc();
                let x = pos.point.x.to_f32();
                let y = (page.content.size.y - y).to_f32();
                dests.push((label, dest_ref));
                loc_to_dest.insert(loc, label);
                chunk
                    .indirect(dest_ref)
                    .start::<Destination>()
                    .page(context.globals.pages[index])
                    .xyz(x, y, None);
            }
        }

        NamedDestinationsOutput { dests, loc_to_dest }
    }

    fn save(context: &mut WriteContext, output: Self::Output) {
        context.dests = output.dests;
        context.loc_to_dest = output.loc_to_dest;
    }
}
