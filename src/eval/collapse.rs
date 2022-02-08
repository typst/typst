use super::{StyleChain, StyleVec, StyleVecBuilder};

/// A wrapper around a [`StyleVecBuilder`] that allows to collapse items.
pub struct CollapsingBuilder<'a, T> {
    builder: StyleVecBuilder<'a, T>,
    staged: Vec<(T, StyleChain<'a>, Option<u8>)>,
    last: Last,
}

/// What the last non-ignorant item was.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Last {
    Weak,
    Destructive,
    Supportive,
}

impl<'a, T: Merge> CollapsingBuilder<'a, T> {
    /// Create a new style-vec builder.
    pub fn new() -> Self {
        Self {
            builder: StyleVecBuilder::new(),
            staged: vec![],
            last: Last::Destructive,
        }
    }

    /// Can only exist when there is at least one supportive item to its left
    /// and to its right, with no destructive items or weak items in between to
    /// its left and no destructive items in between to its right. There may be
    /// ignorant items in between in both directions.
    pub fn weak(&mut self, item: T, strength: u8, styles: StyleChain<'a>) {
        if self.last != Last::Destructive {
            if self.last == Last::Weak {
                if let Some(i) = self
                    .staged
                    .iter()
                    .position(|(.., prev)| prev.map_or(false, |p| p < strength))
                {
                    self.staged.remove(i);
                } else {
                    return;
                }
            }

            self.staged.push((item, styles, Some(strength)));
            self.last = Last::Weak;
        }
    }

    /// Forces nearby weak items to collapse.
    pub fn destructive(&mut self, item: T, styles: StyleChain<'a>) {
        self.flush(false);
        self.push(item, styles);
        self.last = Last::Destructive;
    }

    /// Allows nearby weak items to exist.
    pub fn supportive(&mut self, item: T, styles: StyleChain<'a>) {
        self.flush(true);
        self.push(item, styles);
        self.last = Last::Supportive;
    }

    /// Has no influence on other items.
    pub fn ignorant(&mut self, item: T, styles: StyleChain<'a>) {
        self.staged.push((item, styles, None));
    }

    /// Return the finish style vec and the common prefix chain.
    pub fn finish(mut self) -> (StyleVec<T>, StyleChain<'a>) {
        self.flush(false);
        self.builder.finish()
    }

    /// Push the staged items, filtering out weak items if `supportive` is false.
    fn flush(&mut self, supportive: bool) {
        for (item, styles, strength) in self.staged.drain(..) {
            if supportive || strength.is_none() {
                push_merging(&mut self.builder, item, styles);
            }
        }
    }

    /// Push a new item into the style vector.
    fn push(&mut self, item: T, styles: StyleChain<'a>) {
        push_merging(&mut self.builder, item, styles);
    }
}

/// Push an item into a style-vec builder, trying to merging it with the
/// previous item.
fn push_merging<'a, T: Merge>(
    builder: &mut StyleVecBuilder<'a, T>,
    item: T,
    styles: StyleChain<'a>,
) {
    if let Some((prev_item, prev_styles)) = builder.last_mut() {
        if styles == prev_styles {
            if prev_item.merge(&item) {
                return;
            }
        }
    }

    builder.push(item, styles);
}

impl<'a, T: Merge> Default for CollapsingBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Defines if and how to merge two adjacent items in a [`CollapsingBuilder`].
pub trait Merge {
    /// Try to merge the items, returning whether they were merged.
    ///
    /// Defaults to not merging.
    fn merge(&mut self, next: &Self) -> bool;
}
