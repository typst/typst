//! Element interaction.

use std::borrow::Cow;

use crate::foundations::{
    Behave, Behaviour, Content, StyleChain, StyleVec, StyleVecBuilder,
};

/// A wrapper around a [`StyleVecBuilder`] that allows elements to interact.
#[derive(Debug)]
pub struct BehavedBuilder<'a> {
    /// The internal builder.
    builder: StyleVecBuilder<'a, Cow<'a, Content>>,
    /// Staged weak and ignorant elements that we can't yet commit to the
    /// builder. The option is `Some(_)` for weak elements and `None` for
    /// ignorant elements.
    staged: Vec<(Cow<'a, Content>, Behaviour, StyleChain<'a>)>,
    /// What the last non-ignorant item was.
    last: Behaviour,
}

impl<'a> BehavedBuilder<'a> {
    /// Create a new style-vec builder.
    pub fn new() -> Self {
        Self {
            builder: StyleVecBuilder::new(),
            staged: vec![],
            last: Behaviour::Destructive,
        }
    }

    /// Whether the builder is totally empty.
    pub fn is_empty(&self) -> bool {
        self.builder.is_empty() && self.staged.is_empty()
    }

    /// Whether the builder is empty except for some weak elements that will
    /// probably collapse.
    pub fn has_strong_elements(&self, last: bool) -> bool {
        !self.builder.is_empty()
            || self.staged.iter().any(|(_, behaviour, _)| {
                !matches!(behaviour, Behaviour::Weak(_) | Behaviour::Invisible)
                    || (last && *behaviour == Behaviour::Invisible)
            })
    }

    /// Push an item into the sequence.
    pub fn push(&mut self, elem: Cow<'a, Content>, styles: StyleChain<'a>) {
        let interaction = elem
            .with::<dyn Behave>()
            .map_or(Behaviour::Supportive, Behave::behaviour);

        match interaction {
            Behaviour::Weak(level) => {
                if matches!(self.last, Behaviour::Weak(_)) {
                    let item = elem.with::<dyn Behave>().unwrap();
                    let i = self.staged.iter().position(|prev| {
                        let Behaviour::Weak(prev_level) = prev.1 else { return false };
                        level < prev_level
                            || (level == prev_level && item.larger(prev, styles))
                    });
                    let Some(i) = i else { return };
                    self.staged.remove(i);
                }

                if self.last != Behaviour::Destructive {
                    self.staged.push((elem, interaction, styles));
                    self.last = interaction;
                }
            }
            Behaviour::Supportive => {
                self.flush(true);
                self.builder.push(elem, styles);
                self.last = interaction;
            }
            Behaviour::Destructive => {
                self.flush(false);
                self.builder.push(elem, styles);
                self.last = interaction;
            }
            Behaviour::Ignorant | Behaviour::Invisible => {
                self.staged.push((elem, interaction, styles));
            }
        }
    }

    /// Return the finish style vec and the common prefix chain.
    pub fn finish(mut self) -> (StyleVec<Cow<'a, Content>>, StyleChain<'a>) {
        self.flush(false);
        self.builder.finish()
    }

    /// Push the staged elements, filtering out weak elements if `supportive` is
    /// false.
    fn flush(&mut self, supportive: bool) {
        for (item, interaction, styles) in self.staged.drain(..) {
            if supportive
                || interaction == Behaviour::Ignorant
                || interaction == Behaviour::Invisible
            {
                self.builder.push(item, styles);
            }
        }
    }
}

impl<'a> Default for BehavedBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
