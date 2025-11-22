use ecow::EcoVec;
use unicode_math_class::MathClass;

use crate::foundations::StyleChain;
use crate::math::ir::item::MathItem;
use crate::math::{Limits, MEDIUM, MathSize, THICK, THIN};

/// A processed collection of [`MathItem`]s.
#[derive(Debug, Clone)]
pub struct MathRun<'a> {
    pub(crate) items: EcoVec<MathItem<'a>>,
    pub(crate) styles: StyleChain<'a>,
}

impl<'a> MathRun<'a> {
    pub(crate) fn new<I>(items: I, styles: StyleChain<'a>) -> MathRun<'a>
    where
        I: IntoIterator<Item = MathItem<'a>>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::create(items, styles, false)
    }

    /// Takes the given [`MathItem`]s and do some basic processing.
    pub(crate) fn create<I>(
        items: I,
        styles: StyleChain<'a>,
        closing: bool,
    ) -> MathRun<'a>
    where
        I: IntoIterator<Item = MathItem<'a>>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = items.into_iter();
        let mut resolved = EcoVec::with_capacity(iter.len());
        let iter = iter.peekable();

        let mut last: Option<usize> = None;
        let mut space: Option<MathItem> = None;

        for mut item in iter {
            match item {
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
                        if resolved.is_empty() {
                            continue;
                        }

                        let idx = resolved.len() - 1;
                        if let MathItem::Spacing(prev, true) =
                            &mut resolved.make_mut()[idx]
                        {
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
                    && let Some(s) =
                        spacing(&mut resolved.make_mut()[i], space.take(), &mut item)
                {
                    resolved.insert(i + 1, s);
                }

                last = Some(resolved.len());
            }

            resolved.push(item);
        }

        // Apply closing punctuation spacing if applicable.
        if closing
            && !resolved.is_empty()
            && let idx = resolved.len() - 1
            && let item = &mut resolved.make_mut()[idx]
            && item.rclass() == MathClass::Punctuation
            && item.size().is_none_or(|s| s > MathSize::Script)
        {
            item.set_rspace(Some(THIN))
        } else if let Some(MathItem::Spacing(_, true)) = resolved.last() {
            resolved.pop();
        }

        Self { items: resolved, styles }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MathItem<'a>> {
        self.items.iter()
    }

    pub fn styles(&self) -> StyleChain<'a> {
        self.styles
    }

    pub(crate) fn is_multiline(&self) -> bool {
        self.items.iter().any(|item| matches!(item, MathItem::Linebreak))
    }

    pub(crate) fn class(&self) -> MathClass {
        if self.items.len() == 1 {
            self.items
                .first()
                .map(|item| item.class())
                .unwrap_or(MathClass::Normal)
        } else {
            MathClass::Normal
        }
    }

    pub(crate) fn limits(&self) -> Limits {
        if self.items.len() == 1 {
            self.items.first().map(|item| item.limits()).unwrap_or(Limits::Never)
        } else {
            Limits::Never
        }
    }
}

/// Create the spacing between two items in a given style.
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
