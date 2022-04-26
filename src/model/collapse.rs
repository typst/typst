use super::{StyleChain, StyleVec, StyleVecBuilder};

/// A wrapper around a [`StyleVecBuilder`] that allows to collapse items.
pub struct CollapsingBuilder<'a, T> {
    /// The internal builder.
    builder: StyleVecBuilder<'a, T>,
    /// Staged weak and ignorant items that we can't yet commit to the builder.
    /// The option is `Some(_)` for weak items and `None` for ignorant items.
    staged: Vec<(T, StyleChain<'a>, Option<u8>)>,
    /// What the last non-ignorant item was.
    last: Last,
}

/// What the last non-ignorant item was.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Last {
    Weak,
    Destructive,
    Supportive,
}

impl<'a, T> CollapsingBuilder<'a, T> {
    /// Create a new style-vec builder.
    pub fn new() -> Self {
        Self {
            builder: StyleVecBuilder::new(),
            staged: vec![],
            last: Last::Destructive,
        }
    }

    /// Whether the builder is empty.
    pub fn is_empty(&self) -> bool {
        self.builder.is_empty() && self.staged.is_empty()
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
        self.builder.push(item, styles);
        self.last = Last::Destructive;
    }

    /// Allows nearby weak items to exist.
    pub fn supportive(&mut self, item: T, styles: StyleChain<'a>) {
        self.flush(true);
        self.builder.push(item, styles);
        self.last = Last::Supportive;
    }

    /// Has no influence on other items.
    pub fn ignorant(&mut self, item: T, styles: StyleChain<'a>) {
        self.staged.push((item, styles, None));
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.builder.items().chain(self.staged.iter().map(|(item, ..)| item))
    }

    /// Return the finish style vec and the common prefix chain.
    pub fn finish(mut self) -> (StyleVec<T>, StyleChain<'a>) {
        self.flush(false);
        self.builder.finish()
    }

    /// Push the staged items, filtering out weak items if `supportive` is
    /// false.
    fn flush(&mut self, supportive: bool) {
        for (item, styles, strength) in self.staged.drain(..) {
            if supportive || strength.is_none() {
                self.builder.push(item, styles);
            }
        }
    }
}

impl<'a, T> Default for CollapsingBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
