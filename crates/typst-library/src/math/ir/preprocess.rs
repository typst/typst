use std::ops::{Deref, DerefMut};

use bumpalo::{Bump, boxed::Box as BumpBox, collections::Vec as BumpVec};
use smallvec::SmallVec;
use unicode_math_class::MathClass;

use super::item::MathItem;
use crate::math::{MEDIUM, MathSize, THICK, THIN};

/// Takes the given [`MathItem`]s and processes the spacing between them.
///
/// The `closing` parameter indicates whether a closing delimiter follows the
/// items.
///
/// The behavior of spacing around alignment points is subtle and differs from
/// the `align` environment in amsmath. The current policy is:
/// > always put the correct spacing between fragments separated by an
/// > alignment point, and always put the space on the left of the alignment
/// > point
///
/// For now, it is up to each export target to ensure the above is followed.
pub(crate) fn preprocess<'a, I>(
    items: I,
    bump: &'a Bump,
    closing: bool,
) -> BumpBox<'a, [MathItem<'a>]>
where
    I: IntoIterator<Item = MathItem<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = items.into_iter();
    let mut resolved = MathBuffer::with_capacity(iter.len());
    let iter = iter.peekable();

    let mut last: Option<usize> = None;
    let mut space: Option<MathItem> = None;

    for mut item in iter {
        match item {
            // Tags don't affect layout.
            MathItem::Tag(_) => {
                resolved.push(item);
                continue;
            }

            // Keep space only if supported by spaced items.
            MathItem::Space => {
                if last.is_some() {
                    space = Some(item);
                }
                continue;
            }

            // Explicit spacing disables automatic spacing.
            MathItem::Spacing(width, weak) => {
                last = None;
                space = None;

                if weak {
                    let Some(resolved_last) = resolved.last_mut() else { continue };
                    if let MathItem::Spacing(prev, true) = resolved_last {
                        *prev = (*prev).max(width);
                        continue;
                    }
                }

                resolved.push(item);
                continue;
            }

            // Alignment points are resolved later.
            MathItem::Align => {
                resolved.push(item);
                continue;
            }

            // New line, new things.
            MathItem::Linebreak => {
                resolved.push(item);
                space = None;
                last = None;
                continue;
            }

            _ => {}
        }

        // Convert variable operators into binary operators if something
        // precedes them and they are not preceded by a operator or comparator.
        if item.class() == MathClass::Vary
            && matches!(
                last.map(|i| resolved[i].class()),
                Some(
                    MathClass::Normal
                        | MathClass::Alphabetic
                        | MathClass::Closing
                        | MathClass::Fence
                )
            )
        {
            item.set_class(MathClass::Binary);
        }

        // Insert spacing between the last and this non-ignorant item.
        if !item.is_ignorant() {
            if let Some(i) = last
                && let Some(s) = spacing(&mut resolved[i], space.take(), &mut item)
            {
                resolved.insert(i + 1, s);
            }

            last = Some(resolved.len());
        }

        resolved.push(item);
    }

    // Apply closing punctuation spacing if applicable.
    if closing
        && let Some(item) = resolved.last_mut()
        && item.rclass() == MathClass::Punctuation
        && item.size().is_none_or(|s| s > MathSize::Script)
    {
        item.set_rspace(Some(THIN))
    } else if let Some(idx) = resolved.last_index()
        && let MathItem::Spacing(_, true) = resolved.0[idx]
    {
        resolved.0.remove(idx);
    }

    BumpVec::from_iter_in(resolved.0, bump).into_boxed_slice()
}

/// Computes the spacing between two adjacent math items.
fn spacing<'a>(
    l: &mut MathItem,
    space: Option<MathItem<'a>>,
    r: &mut MathItem,
) -> Option<MathItem<'a>> {
    use MathClass::*;

    let script = |f: &MathItem| f.size().is_some_and(|s| s <= MathSize::Script);

    match (l.rclass(), r.lclass()) {
        // No spacing before punctuation; thin spacing after punctuation, unless
        // in script size.
        (_, Punctuation) => {}
        (Punctuation, _) if !script(l) => l.set_rspace(Some(THIN)),

        // No spacing after opening delimiters and before closing delimiters.
        (Opening, _) | (_, Closing) => {}

        // Thick spacing around relations, unless followed by a another relation
        // or in script size.
        (Relation, Relation) => {}
        (Relation, _) if !script(l) => l.set_rspace(Some(THICK)),
        (_, Relation) if !script(r) => r.set_lspace(Some(THICK)),

        // Medium spacing around binary operators, unless in script size.
        (Binary, _) if !script(l) => l.set_rspace(Some(MEDIUM)),
        (_, Binary) if !script(r) => r.set_lspace(Some(MEDIUM)),

        // Thin spacing around large operators, unless to the left of
        // an opening delimiter. TeXBook, p170
        (Large, Opening | Fence) => {}
        (Large, _) => l.set_rspace(Some(THIN)),

        (_, Large) => r.set_lspace(Some(THIN)),

        // Spacing around spaced frames.
        _ if (l.is_spaced() || r.is_spaced()) => return space,

        _ => {}
    };

    None
}

/// A wrapper around `SmallVec<[MathItem; 8]>` that ignores ignorant items in
/// some access methods.
struct MathBuffer<'a>(SmallVec<[MathItem<'a>; 8]>);

impl<'a> MathBuffer<'a> {
    /// Creates a new buffer with the given capacity.
    fn with_capacity(size: usize) -> Self {
        Self(SmallVec::with_capacity(size))
    }

    /// Returns a mutable reference to the last non-ignorant item.
    fn last_mut(&mut self) -> Option<&mut MathItem<'a>> {
        self.0.iter_mut().rev().find(|i| !i.is_ignorant())
    }

    /// Returns the physical index of the last non-ignorant item.
    fn last_index(&self) -> Option<usize> {
        self.0.iter().rposition(|i| !i.is_ignorant())
    }
}

impl<'a> Deref for MathBuffer<'a> {
    type Target = SmallVec<[MathItem<'a>; 8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for MathBuffer<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
