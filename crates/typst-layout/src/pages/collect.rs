use rustc_hash::FxHashSet;
use typst_library::foundations::StyleChain;
use typst_library::introspection::{Locator, SplitLocator, Tag, TagElem};
use typst_library::layout::{PagebreakElem, Parity};
use typst_library::routines::Pair;

/// An item in page layout.
pub enum Item<'a> {
    /// A page run containing content. All runs will be layouted in parallel.
    Run(&'a [Pair<'a>], StyleChain<'a>, Locator<'a>),
    /// Tags in between pages. These will be prepended to the first start of
    /// the next page, or appended at the very end of the final page if there is
    /// no next page.
    Tags(&'a [Pair<'a>]),
    /// An instruction to possibly add a page to bring the page number parity to
    /// the desired state. Can only be done at the end, sequentially, because it
    /// requires knowledge of the concrete page number.
    Parity(Parity, StyleChain<'a>, Locator<'a>),
}

/// Slices up the children into logical parts, processing styles and handling
/// things like tags and weak pagebreaks.
pub fn collect<'a>(
    mut children: &'a mut [Pair<'a>],
    locator: &mut SplitLocator<'a>,
    mut initial: StyleChain<'a>,
) -> Vec<Item<'a>> {
    // The collected page-level items.
    let mut items: Vec<Item<'a>> = vec![];
    // When this is true, an empty page should be added to `pages` at the end.
    let mut staged_empty_page = true;

    // The `children` are a flat list of flow-level items and pagebreaks. This
    // loops splits it up into pagebreaks and consecutive slices of
    // non-pagebreaks. From these pieces, we build page items that we can then
    // layout in parallel.
    while let Some(&(elem, styles)) = children.first() {
        if let Some(pagebreak) = elem.to_packed::<PagebreakElem>() {
            // Add a blank page if we encounter a strong pagebreak and there was
            // a staged empty page.
            let strong = !pagebreak.weak.get(styles);
            if strong && staged_empty_page {
                let locator = locator.next(&elem.span());
                items.push(Item::Run(&[], initial, locator));
            }

            // Add an instruction to adjust the page parity if requested.
            if let Some(parity) = pagebreak.to.get(styles) {
                let locator = locator.next(&elem.span());
                items.push(Item::Parity(parity, styles, locator));
            }

            // The initial styles for the next page are ours unless this is a
            // "boundary" pagebreak. Such a pagebreak is generated at the end of
            // the scope of a page set rule to ensure a page boundary. Its
            // styles correspond to the styles _before_ the page set rule, so we
            // don't want to apply it to a potential empty page.
            if !pagebreak.boundary.get(styles) {
                initial = styles;
            }

            // Stage an empty page after a strong pagebreak.
            staged_empty_page |= strong;

            // Advance to the next child.
            children = &mut children[1..];
        } else {
            // Find the end of the consecutive non-pagebreak run.
            let end =
                children.iter().take_while(|(c, _)| !c.is::<PagebreakElem>()).count();

            // Migrate start tags without accompanying end tags from before a
            // pagebreak to after it.
            let end = migrate_unterminated_tags(children, end);
            if end == 0 {
                continue;
            }

            // Advance to the rest of the children.
            let (group, rest) = children.split_at_mut(end);
            children = rest;

            // If all that is left now are tags, then we don't want to add a
            // page just for them (since no group would have been detected in a
            // tagless layout and tags should never affect the layout). For this
            // reason, we remember them in a `PageItem::Tags` and later insert
            // them at the _very start_ of the next page, even before the
            // header.
            //
            // We don't do this if all that's left is end boundary pagebreaks
            // and if an empty page is still staged, since then we can just
            // conceptually replace that final page with us.
            if group.iter().all(|(c, _)| c.is::<TagElem>())
                && !(staged_empty_page
                    && children.iter().all(|&(c, s)| {
                        c.to_packed::<PagebreakElem>().is_some_and(|c| c.boundary.get(s))
                    }))
            {
                items.push(Item::Tags(group));
                continue;
            }

            // Record a page run and then disregard a staged empty page because
            // we have real content now.
            let locator = locator.next(&elem.span());
            items.push(Item::Run(group, initial, locator));
            staged_empty_page = false;
        }
    }

    // Flush a staged empty page.
    if staged_empty_page {
        items.push(Item::Run(&[], initial, locator.next(&())));
    }

    items
}

/// Migrates trailing start tags without accompanying end tags from before
/// a pagebreak to after it. Returns the position right after the last
/// non-migrated tag.
///
/// This is important because we want the positions of introspectable elements
/// that technically started before a pagebreak, but have no visible content
/// yet, to be after the pagebreak. A typical case where this happens is `show
/// heading: it => pagebreak() + it`.
fn migrate_unterminated_tags(children: &mut [Pair], mid: usize) -> usize {
    let (before, after) = children.split_at(mid);
    let start = mid - before.iter().rev().take_while(|&(c, _)| c.is::<TagElem>()).count();
    let end = mid + after.iter().take_while(|&(c, _)| c.is::<PagebreakElem>()).count();

    // Rileviamo i tag posizionati prima del punto di interruzione
    let mut excluded = rustc_hash::FxHashSet::default();
    let mut saw_unterminated = false;

    for (c, _) in children[start..mid].iter() {
        if let Some(elem) = c.to_packed::<TagElem>() {
            match elem.tag {
                Tag::Start(..) => {
                    // Trovato un tag di apertura: da qui in poi i tag conclusi
                    // successivi non devono essere esclusi dalla migrazione
                    saw_unterminated = true;
                }
                Tag::End(loc, ..) => {
                    // Se non abbiamo incontrato tag aperti prima, questo tag finale
                    // fa parte di una coppia già chiusa e non va migrato
                    if !saw_unterminated {
                        excluded.insert(loc);
                    }
                }
            }
        }
    }

    let key = |(c, _): &Pair| match c.to_packed::<TagElem>() {
        Some(elem) => {
            if excluded.contains(&elem.tag.location()) {
                -1
            } else {
                1
            }
        }
        None => 0,
    };

    children[start..end].sort_by_key(key);
    start + children[start..end].iter().take_while(|pair| key(pair) == -1).count()
}
