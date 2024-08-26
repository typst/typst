//! Element interaction.

use crate::foundations::{Content, StyleChain};

/// How an element interacts with other elements in a stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Behaviour {
    /// A weak element which only survives when a supportive element is before
    /// and after it. Furthermore, per consecutive run of weak elements, only
    /// one survives: The one with the lowest weakness level (or the larger one
    /// if there is a tie).
    Weak(usize),
    /// An element that enables adjacent weak elements to exist. The default.
    Supportive,
    /// An element that destroys adjacent weak elements.
    Destructive,
    /// An element that does not interact at all with other elements, having the
    /// same effect on them as if it didn't exist.
    Ignorant,
}

impl Behaviour {
    /// Whether this of `Weak(_)` variant.
    pub fn is_weak(self) -> bool {
        matches!(self, Self::Weak(_))
    }
}

/// How the element interacts with other elements.
pub trait Behave {
    /// The element's interaction behaviour.
    fn behaviour(&self) -> Behaviour;

    /// Whether this weak element is larger than a previous one and thus picked
    /// as the maximum when the levels are the same.
    #[allow(unused_variables)]
    fn larger(&self, prev: &(&Content, StyleChain), styles: StyleChain) -> bool {
        false
    }
}

/// Processes a sequence of content and resolves behaviour interactions between
/// them and separates local styles for each element from the shared trunk of
/// styles.
#[derive(Debug)]
pub struct BehavedBuilder<'a> {
    /// The collected content with its styles.
    buf: Vec<(&'a Content, StyleChain<'a>)>,
    /// What the last non-ignorant, visible item was.
    last: Behaviour,
}

impl<'a> BehavedBuilder<'a> {
    /// Create a new style-vec builder.
    pub fn new() -> Self {
        Self { buf: vec![], last: Behaviour::Destructive }
    }

    /// Whether the builder is totally empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Push an item into the builder.
    pub fn push(&mut self, content: &'a Content, styles: StyleChain<'a>) {
        let mut behaviour = content.behaviour();
        match behaviour {
            Behaviour::Supportive => {}
            Behaviour::Weak(level) => match self.last {
                // Remove either this or the preceding weak item.
                Behaviour::Weak(prev_level) => {
                    if level > prev_level {
                        return;
                    }

                    let i = self.find_last_weak().unwrap();
                    if level == prev_level
                        && !content
                            .with::<dyn Behave>()
                            .unwrap()
                            .larger(&self.buf[i], styles)
                    {
                        return;
                    }

                    self.buf.remove(i);
                }
                Behaviour::Destructive => return,
                _ => {}
            },
            Behaviour::Destructive => {
                // Remove preceding weak item.
                if self.last.is_weak() {
                    let i = self.find_last_weak().unwrap();
                    self.buf.remove(i);
                }
            }
            Behaviour::Ignorant => {
                behaviour = self.last;
            }
        }

        self.last = behaviour;
        self.buf.push((content, styles));
    }

    /// Iterate over the content that has been pushed so far.
    pub fn items(&self) -> impl Iterator<Item = &'a Content> + '_ {
        self.buf.iter().map(|&(c, _)| c)
    }

    /// Return the built content (possibly styled with local styles) plus a
    /// trunk style chain and a span for the collection.
    pub fn finish(mut self) -> Vec<(&'a Content, StyleChain<'a>)> {
        self.trim_weak();
        self.buf
    }

    /// Trim a possibly remaining weak item.
    fn trim_weak(&mut self) {
        if self.last.is_weak() {
            let i = self.find_last_weak().unwrap();
            self.buf.remove(i);
        }
    }

    /// Get the position of the right most weak item.
    fn find_last_weak(&self) -> Option<usize> {
        self.buf.iter().rposition(|(c, _)| c.behaviour().is_weak())
    }
}

impl<'a> Default for BehavedBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
