//! Node interaction.

use typst::model::{capability, Content, StyleChain, StyleVec, StyleVecBuilder};

/// How a node interacts with other nodes.
#[capability]
pub trait Behave: 'static + Send + Sync {
    /// The node's interaction behaviour.
    fn behaviour(&self) -> Behaviour;

    /// Whether this weak node is larger than a previous one and thus picked as
    /// the maximum when the levels are the same.
    #[allow(unused_variables)]
    fn larger(&self, prev: &Content) -> bool {
        false
    }
}

/// How a node interacts with other nodes in a stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Behaviour {
    /// A weak node which only survives when a supportive node is before and
    /// after it. Furthermore, per consecutive run of weak nodes, only one
    /// survives: The one with the lowest weakness level (or the larger one if
    /// there is a tie).
    Weak(u8),
    /// A node that enables adjacent weak nodes to exist. The default.
    Supportive,
    /// A node that destroys adjacent weak nodes.
    Destructive,
    /// A node that does not interact at all with other nodes, having the
    /// same effect as if it didn't exist.
    Ignorant,
}

/// A wrapper around a [`StyleVecBuilder`] that allows items to interact.
pub struct BehavedBuilder<'a> {
    /// The internal builder.
    builder: StyleVecBuilder<'a, Content>,
    /// Staged weak and ignorant items that we can't yet commit to the builder.
    /// The option is `Some(_)` for weak items and `None` for ignorant items.
    staged: Vec<(Content, Behaviour, StyleChain<'a>)>,
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

    /// Whether the builder is empty.
    pub fn is_empty(&self) -> bool {
        self.builder.is_empty() && self.staged.is_empty()
    }

    /// Push an item into the sequence.
    pub fn push(&mut self, item: Content, styles: StyleChain<'a>) {
        let interaction = item
            .to::<dyn Behave>()
            .map_or(Behaviour::Supportive, Behave::behaviour);

        match interaction {
            Behaviour::Weak(level) => {
                if matches!(self.last, Behaviour::Weak(_)) {
                    let item = item.to::<dyn Behave>().unwrap();
                    let i = self.staged.iter().position(|prev| {
                        let Behaviour::Weak(prev_level) = prev.1 else { return false };
                        level < prev_level
                            || (level == prev_level && item.larger(&prev.0))
                    });
                    let Some(i) = i else { return };
                    self.staged.remove(i);
                }

                if self.last != Behaviour::Destructive {
                    self.staged.push((item, interaction, styles));
                    self.last = interaction;
                }
            }
            Behaviour::Supportive => {
                self.flush(true);
                self.builder.push(item, styles);
                self.last = interaction;
            }
            Behaviour::Destructive => {
                self.flush(false);
                self.builder.push(item, styles);
                self.last = interaction;
            }
            Behaviour::Ignorant => {
                self.staged.push((item, interaction, styles));
            }
        }
    }

    /// Iterate over the contained items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = &Content> {
        self.builder.items().chain(self.staged.iter().map(|(item, ..)| item))
    }

    /// Return the finish style vec and the common prefix chain.
    pub fn finish(mut self) -> (StyleVec<Content>, StyleChain<'a>) {
        self.flush(false);
        self.builder.finish()
    }

    /// Push the staged items, filtering out weak items if `supportive` is
    /// false.
    fn flush(&mut self, supportive: bool) {
        for (item, interaction, styles) in self.staged.drain(..) {
            if supportive || interaction == Behaviour::Ignorant {
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
