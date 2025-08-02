//! Source file management.

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::Arc;

use typst_utils::LazyHash;

use crate::lines::Lines;
use crate::reparser::reparse;
use crate::{FileId, LinkedNode, Span, SyntaxNode, VirtualPath, parse};

/// A source file.
///
/// All line and column indices start at zero, just like byte indices. Only for
/// user-facing display, you should add 1 to them.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone)]
pub struct Source(Arc<Repr>);

/// The internal representation.
#[derive(Clone)]
struct Repr {
    id: FileId,
    root: LazyHash<SyntaxNode>,
    lines: LazyHash<Lines<String>>,
}

impl Source {
    /// Create a new source file.
    pub fn new(id: FileId, text: String) -> Self {
        let _scope = typst_timing::TimingScope::new("create source");
        let mut root = parse(&text);
        root.numberize(id, Span::FULL).unwrap();
        Self(Arc::new(Repr {
            id,
            lines: LazyHash::new(Lines::new(text)),
            root: LazyHash::new(root),
        }))
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(text: impl Into<String>) -> Self {
        Self::new(FileId::new(None, VirtualPath::new("main.typ")), text.into())
    }

    /// The root node of the file's untyped syntax tree.
    pub fn root(&self) -> &SyntaxNode {
        &self.0.root
    }

    /// The id of the source file.
    pub fn id(&self) -> FileId {
        self.0.id
    }

    /// The whole source as a string slice.
    pub fn text(&self) -> &str {
        self.0.lines.text()
    }

    /// An acceleration structure for conversion of UTF-8, UTF-16 and
    /// line/column indices.
    pub fn lines(&self) -> &Lines<String> {
        &self.0.lines
    }

    /// Fully replace the source text.
    ///
    /// This performs a naive (suffix/prefix-based) diff of the old and new text
    /// to produce the smallest single edit that transforms old into new and
    /// then calls [`edit`](Self::edit) with it.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    pub fn replace(&mut self, new: &str) -> Range<usize> {
        let _scope = typst_timing::TimingScope::new("replace source");

        let Some((prefix, suffix)) = self.0.lines.replacement_range(new) else {
            return 0..0;
        };

        let old = self.text();
        let replace = prefix..old.len() - suffix;
        let with = &new[prefix..new.len() - suffix];
        self.edit(replace, with)
    }

    /// Edit the source file by replacing the given range.
    ///
    /// Returns the range in the new source that was ultimately reparsed.
    ///
    /// The method panics if the `replace` range is out of bounds.
    #[track_caller]
    pub fn edit(&mut self, replace: Range<usize>, with: &str) -> Range<usize> {
        let inner = Arc::make_mut(&mut self.0);

        // Update the text and lines.
        inner.lines.edit(replace.clone(), with);

        // Incrementally reparse the replaced range.
        reparse(&mut inner.root, inner.lines.text(), replace, with.len())
    }

    /// Find the node with the given span.
    ///
    /// Returns `None` if the span does not point into this source file.
    pub fn find(&self, span: Span) -> Option<LinkedNode<'_>> {
        LinkedNode::new(self.root()).find(span)
    }

    /// The byte range for a span in this file up to the end of a number of
    /// sibling nodes.
    ///
    /// Returns `None` if the span does not point into this source file. Panics
    /// if the span is not followed by the expected number of siblings.
    ///
    /// Typically, it's easier to use `WorldExt::range` instead.
    pub fn range(&self, span: Span, plus: Option<NonZeroUsize>) -> Option<Range<usize>> {
        let node = self.find(span)?;
        if let Some(n) = plus {
            let start = node.range().start;
            let nth_sibling = node.siblings().unwrap().nth(n.get() - 1).unwrap();
            let end = nth_sibling.range().end;
            Some(start..end)
        } else {
            Some(node.range())
        }
    }

    /// The byte offset of the start of the given span in this file.
    pub fn start_offset(&self, span: Span) -> Option<usize> {
        Some(self.find(span)?.range().start)
    }
}

impl Debug for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Source({:?})", self.id().vpath())
    }
}

impl Hash for Source {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
        self.0.lines.hash(state);
        self.0.root.hash(state);
    }
}

impl AsRef<str> for Source {
    fn as_ref(&self) -> &str {
        self.text()
    }
}
