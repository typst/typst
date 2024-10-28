use std::collections::HashSet;

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
    mut locator: SplitLocator<'a>,
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
            let strong = !pagebreak.weak(styles);
            if strong && staged_empty_page {
                let locator = locator.next(&elem.span());
                items.push(Item::Run(&[], initial, locator));
            }

            // Add an instruction to adjust the page parity if requested.
            if let Some(parity) = pagebreak.to(styles) {
                let locator = locator.next(&elem.span());
                items.push(Item::Parity(parity, styles, locator));
            }

            // The initial styles for the next page are ours unless this is a
            // "boundary" pagebreak. Such a pagebreak is generated at the end of
            // the scope of a page set rule to ensure a page boundary. It's
            // styles correspond to the styles _before_ the page set rule, so we
            // don't want to apply it to a potential empty page.
            if !pagebreak.boundary(styles) {
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
                        c.to_packed::<PagebreakElem>().is_some_and(|c| c.boundary(s))
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
    // Compute the range from before the first trailing tag to after the last
    // following pagebreak.
    let (before, after) = children.split_at(mid);
    let start = mid - before.iter().rev().take_while(|&(c, _)| c.is::<TagElem>()).count();
    let end = mid + after.iter().take_while(|&(c, _)| c.is::<PagebreakElem>()).count();

    // Determine the set of tag locations which we won't migrate (because they
    // are terminated).
    let excluded: HashSet<_> = children[start..mid]
        .iter()
        .filter_map(|(c, _)| match c.to_packed::<TagElem>()?.tag {
            Tag::Start(_) => None,
            Tag::End(loc, _) => Some(loc),
        })
        .collect();

    // A key function that partitions the area of interest into three groups:
    // Excluded tags (-1) | Pagebreaks (0) | Migrated tags (1).
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

    // Partition the children using a *stable* sort. While it would be possible
    // to write a more efficient direct algorithm for this, the sort version is
    // less likely to have bugs and this is absolutely not on a hot path.
    children[start..end].sort_by_key(key);

    // Compute the new end index, right before the pagebreaks.
    start + children[start..end].iter().take_while(|pair| key(pair) == -1).count()
}
