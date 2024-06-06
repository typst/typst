//! Element interaction.

use std::fmt::{Debug, Formatter};

use ecow::EcoVec;

use crate::foundations::{Content, StyleChain, Styles};
use crate::syntax::Span;

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
    /// same effect as if it didn't exist, but has layout extent and/or a visual
    /// representation.
    Ignorant,
    /// An element that does not have any layout extent or visual
    /// representation.
    Invisible,
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

    /// Whether the builder has any proper (non-weak & visible) elements.
    pub fn has_strong_elements(&self, last: bool) -> bool {
        self.buf.iter().any(|(content, _)| {
            let behaviour = content.behaviour();
            !matches!(behaviour, Behaviour::Weak(_) | Behaviour::Invisible)
                || (last && behaviour == Behaviour::Invisible)
        })
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
            Behaviour::Ignorant | Behaviour::Invisible => {
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
    pub fn finish(mut self) -> (StyleVec, StyleChain<'a>, Span) {
        self.trim_weak();

        let span = self.determine_span();
        let (trunk, depth) = self.determine_style_trunk();

        let mut elements = EcoVec::with_capacity(self.buf.len());
        let mut styles = EcoVec::<(Styles, usize)>::new();
        let mut last: Option<(StyleChain<'a>, usize)> = None;

        for (element, chain) in self.buf.into_iter() {
            elements.push(element.clone());

            if let Some((prev, run)) = &mut last {
                if chain == *prev {
                    *run += 1;
                } else {
                    styles.push((prev.suffix(depth), *run));
                    last = Some((chain, 1));
                }
            } else {
                last = Some((chain, 1));
            }
        }

        if let Some((last, run)) = last {
            let skippable = styles.is_empty() && last == trunk;
            if !skippable {
                styles.push((last.suffix(depth), run));
            }
        }

        (StyleVec { elements, styles }, trunk, span)
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

    /// Determine a span for the built collection.
    fn determine_span(&self) -> Span {
        let mut span = Span::detached();
        for &(content, _) in &self.buf {
            span = content.span();
            if !span.is_detached() {
                break;
            }
        }
        span
    }

    /// Determine the shared trunk style chain.
    fn determine_style_trunk(&self) -> (StyleChain<'a>, usize) {
        // Determine shared style depth and first span.
        let mut trunk = match self.buf.first() {
            Some(&(_, chain)) => chain,
            None => Default::default(),
        };

        let mut depth = trunk.links().count();
        for (_, mut chain) in &self.buf {
            let len = chain.links().count();
            if len < depth {
                for _ in 0..depth - len {
                    trunk.pop();
                }
                depth = len;
            } else if len > depth {
                for _ in 0..len - depth {
                    chain.pop();
                }
            }

            while depth > 0 && chain != trunk {
                trunk.pop();
                chain.pop();
                depth -= 1;
            }
        }

        (trunk, depth)
    }
}

impl<'a> Default for BehavedBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// A sequence of elements with associated styles.
#[derive(Clone, PartialEq, Hash)]
pub struct StyleVec {
    /// The elements themselves.
    elements: EcoVec<Content>,
    /// A run-length encoded list of style lists.
    ///
    /// Each element is a (styles, count) pair. Any elements whose
    /// style falls after the end of this list is considered to
    /// have an empty style list.
    styles: EcoVec<(Styles, usize)>,
}

impl StyleVec {
    /// Create a style vector from an unstyled vector content.
    pub fn wrap(elements: EcoVec<Content>) -> Self {
        Self { elements, styles: EcoVec::new() }
    }

    /// Whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// The raw, unstyled elements.
    pub fn elements(&self) -> &[Content] {
        &self.elements
    }

    /// Get a style property, but only if it is the same for all children of the
    /// style vector.
    pub fn shared_get<T: PartialEq>(
        &self,
        styles: StyleChain<'_>,
        getter: fn(StyleChain) -> T,
    ) -> Option<T> {
        let value = getter(styles);
        self.styles
            .iter()
            .all(|(local, _)| getter(styles.chain(local)) == value)
            .then_some(value)
    }

    /// Iterate over the contained content and style chains.
    pub fn chain<'a>(
        &'a self,
        outer: &'a StyleChain<'_>,
    ) -> impl Iterator<Item = (&'a Content, StyleChain<'a>)> {
        self.iter().map(|(element, local)| (element, outer.chain(local)))
    }

    /// Iterate over pairs of content and styles.
    pub fn iter(&self) -> impl Iterator<Item = (&Content, &Styles)> {
        static EMPTY: Styles = Styles::new();
        self.elements.iter().zip(
            self.styles
                .iter()
                .flat_map(|(local, count)| std::iter::repeat(local).take(*count))
                .chain(std::iter::repeat(&EMPTY)),
        )
    }

    /// Iterate over pairs of content and styles.
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl Iterator<Item = (Content, Styles)> {
        self.elements.into_iter().zip(
            self.styles
                .into_iter()
                .flat_map(|(local, count)| std::iter::repeat(local).take(count))
                .chain(std::iter::repeat(Styles::new())),
        )
    }
}

impl Debug for StyleVec {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list()
            .entries(self.iter().map(|(element, local)| {
                typst_utils::debug(|f| {
                    for style in local.iter() {
                        writeln!(f, "#{style:?}")?;
                    }
                    element.fmt(f)
                })
            }))
            .finish()
    }
}
