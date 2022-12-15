use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, Range};
use std::rc::Rc;

use super::{SyntaxKind, SyntaxNode};

/// A syntax node in a context.
///
/// Knows its exact offset in the file and provides access to its
/// children, parent and siblings.
///
/// **Note that all sibling and leaf accessors skip over trivia!**
#[derive(Clone)]
pub struct LinkedNode<'a> {
    node: &'a SyntaxNode,
    parent: Option<Rc<Self>>,
    index: usize,
    offset: usize,
}

impl<'a> LinkedNode<'a> {
    /// Start a new traversal at the source's root node.
    pub fn new(root: &'a SyntaxNode) -> Self {
        Self { node: root, parent: None, index: 0, offset: 0 }
    }

    /// Get the contained syntax node.
    pub fn get(&self) -> &'a SyntaxNode {
        self.node
    }

    /// The index of this node in its parent's children list.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The absolute byte offset of the this node in the source file.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The byte range of the this node in the source file.
    pub fn range(&self) -> Range<usize> {
        self.offset..self.offset + self.node.len()
    }

    /// Get this node's children.
    pub fn children(&self) -> LinkedChildren<'a> {
        LinkedChildren {
            parent: Rc::new(self.clone()),
            iter: self.node.children().enumerate(),
            front: self.offset,
            back: self.offset + self.len(),
        }
    }
}

/// Access to parents and siblings.
impl<'a> LinkedNode<'a> {
    /// Get this node's parent.
    pub fn parent(&self) -> Option<&Self> {
        self.parent.as_deref()
    }

    /// Get the kind of this node's parent.
    pub fn parent_kind(&self) -> Option<&'a SyntaxKind> {
        Some(self.parent()?.node.kind())
    }

    /// Get the first previous non-trivia sibling node.
    pub fn prev_sibling(&self) -> Option<Self> {
        let parent = self.parent()?;
        let index = self.index.checked_sub(1)?;
        let node = parent.node.children().nth(index)?;
        let offset = self.offset - node.len();
        let prev = Self { node, parent: self.parent.clone(), index, offset };
        if prev.kind().is_trivia() {
            prev.prev_sibling()
        } else {
            Some(prev)
        }
    }

    /// Get the next non-trivia sibling node.
    pub fn next_sibling(&self) -> Option<Self> {
        let parent = self.parent()?;
        let index = self.index.checked_add(1)?;
        let node = parent.node.children().nth(index)?;
        let offset = self.offset + self.node.len();
        let next = Self { node, parent: self.parent.clone(), index, offset };
        if next.kind().is_trivia() {
            next.next_sibling()
        } else {
            Some(next)
        }
    }
}

/// Access to leafs.
impl<'a> LinkedNode<'a> {
    /// Get the rightmost non-trivia leaf before this node.
    pub fn prev_leaf(&self) -> Option<Self> {
        let mut node = self.clone();
        while let Some(prev) = node.prev_sibling() {
            if let Some(leaf) = prev.rightmost_leaf() {
                return Some(leaf);
            }
            node = prev;
        }
        self.parent()?.prev_leaf()
    }

    /// Find the leftmost contained non-trivia leaf.
    pub fn leftmost_leaf(&self) -> Option<Self> {
        if self.is_leaf() && !self.kind().is_trivia() && !self.kind().is_error() {
            return Some(self.clone());
        }

        for child in self.children() {
            if let Some(leaf) = child.leftmost_leaf() {
                return Some(leaf);
            }
        }

        None
    }

    /// Get the leaf at the specified cursor position.
    pub fn leaf_at(&self, cursor: usize) -> Option<Self> {
        if self.node.children().len() == 0 && cursor <= self.offset + self.len() {
            return Some(self.clone());
        }

        let mut offset = self.offset;
        let count = self.node.children().len();
        for (i, child) in self.children().enumerate() {
            let len = child.len();
            if (offset < cursor && cursor <= offset + len)
                || (offset == cursor && i + 1 == count)
            {
                return child.leaf_at(cursor);
            }
            offset += len;
        }

        None
    }

    /// Find the rightmost contained non-trivia leaf.
    pub fn rightmost_leaf(&self) -> Option<Self> {
        if self.is_leaf() && !self.kind().is_trivia() {
            return Some(self.clone());
        }

        for child in self.children().rev() {
            if let Some(leaf) = child.rightmost_leaf() {
                return Some(leaf);
            }
        }

        None
    }

    /// Get the leftmost non-trivia leaf after this node.
    pub fn next_leaf(&self) -> Option<Self> {
        let mut node = self.clone();
        while let Some(next) = node.next_sibling() {
            if let Some(leaf) = next.leftmost_leaf() {
                return Some(leaf);
            }
            node = next;
        }
        self.parent()?.next_leaf()
    }
}

impl Deref for LinkedNode<'_> {
    type Target = SyntaxNode;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl Debug for LinkedNode<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

/// An iterator over the children of a linked node.
pub struct LinkedChildren<'a> {
    parent: Rc<LinkedNode<'a>>,
    iter: std::iter::Enumerate<std::slice::Iter<'a, SyntaxNode>>,
    front: usize,
    back: usize,
}

impl<'a> Iterator for LinkedChildren<'a> {
    type Item = LinkedNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(index, node)| {
            let offset = self.front;
            self.front += node.len();
            LinkedNode {
                node,
                parent: Some(self.parent.clone()),
                index,
                offset,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for LinkedChildren<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(index, node)| {
            self.back -= node.len();
            LinkedNode {
                node,
                parent: Some(self.parent.clone()),
                index,
                offset: self.back,
            }
        })
    }
}

impl ExactSizeIterator for LinkedChildren<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Source;

    #[test]
    fn test_linked_node() {
        let source = Source::detached("#set text(12pt, red)");

        // Find "text".
        let node = LinkedNode::new(source.root()).leaf_at(7).unwrap();
        assert_eq!(node.offset(), 5);
        assert_eq!(node.len(), 4);
        assert_eq!(node.kind(), &SyntaxKind::Ident("text".into()));

        // Go back to "#set". Skips the space.
        let prev = node.prev_sibling().unwrap();
        assert_eq!(prev.offset(), 0);
        assert_eq!(prev.len(), 4);
        assert_eq!(prev.kind(), &SyntaxKind::Set);
    }

    #[test]
    fn test_linked_node_non_trivia_leaf() {
        let source = Source::detached("#set fun(12pt, red)");
        let leaf = LinkedNode::new(source.root()).leaf_at(6).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        assert_eq!(leaf.kind(), &SyntaxKind::Ident("fun".into()));
        assert_eq!(prev.kind(), &SyntaxKind::Set);

        let source = Source::detached("#let x = 10");
        let leaf = LinkedNode::new(source.root()).leaf_at(9).unwrap();
        let prev = leaf.prev_leaf().unwrap();
        let next = leaf.next_leaf().unwrap();
        assert_eq!(prev.kind(), &SyntaxKind::Eq);
        assert_eq!(leaf.kind(), &SyntaxKind::Space { newlines: 0 });
        assert_eq!(next.kind(), &SyntaxKind::Int(10));
    }
}
