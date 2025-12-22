//! Source file management.

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
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
pub struct Source(Arc<SourceInner>);

/// The internal representation of a [`Source`].
#[derive(Clone)]
struct SourceInner {
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
        Self(Arc::new(SourceInner {
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

    /// Get the byte range for the given span in this file.
    ///
    /// Returns `None` if the span does not point into this source file.
    ///
    /// Typically, it's easier to use `WorldExt::range` instead.
    pub fn range(&self, span: Span) -> Option<Range<usize>> {
        Some(self.find(span)?.range())
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
