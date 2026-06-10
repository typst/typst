//! Source file management.

use std::fmt::{self, Debug, Formatter};
use std::ops::Range;
use std::sync::Arc;

use typst_utils::LazyHash;

use crate::lines::Lines;
use crate::reparser::reparse;
use crate::{
    FileId, LinkedNode, RootedPath, Span, SpanNumber, SubRange, SyntaxNode, VirtualPath,
    VirtualRoot, parse,
};

/// A Typst source file containing the full source text, a mapping from byte
/// indices to lines/columns, and the parsed syntax tree.
///
/// All line and column indices start at zero, just like byte indices. Only for
/// user-facing display, you should add 1 to them.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Hash)]
pub struct Source(Arc<LazyHash<SourceInner>>);

/// The internal representation of a [`Source`].
#[derive(Clone, Hash)]
struct SourceInner {
    id: FileId,
    root: SyntaxNode,
    lines: Lines<String>,
}

impl Source {
    /// Create a new source file.
    pub fn new(id: FileId, text: String) -> Self {
        let _scope = typst_timing::TimingScope::new("create source");
        let mut root = parse(&text);
        root.numberize(id, Span::FULL).unwrap();
        Self(Arc::new(LazyHash::new(SourceInner { id, lines: Lines::new(text), root })))
    }

    /// Create a source file without a real id and path, usually for testing.
    pub fn detached(text: impl Into<String>) -> Self {
        Self::new(
            RootedPath::new(VirtualRoot::Project, VirtualPath::new("main.typ").unwrap())
                .intern(),
            text.into(),
        )
    }

    /// Create a new source file with an already created syntax tree.
    pub fn with_root(id: FileId, text: String, root: SyntaxNode) -> Self {
        Self(Arc::new(LazyHash::new(SourceInner { id, lines: Lines::new(text), root })))
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
        let inner = &mut **Arc::make_mut(&mut self.0);

        // Update the text and lines.
        inner.lines.edit(replace.clone(), with);

        // Incrementally reparse the replaced range.
        reparse(&mut inner.root, inner.lines.text(), replace, with.len())
    }

    /// Find the node with the given span.
    ///
    /// Returns `None` if the span does not point into this source file.
    pub fn find(&self, span: Span) -> Option<LinkedNode<'_>> {
        if span.id() != Some(self.id()) {
            return None;
        }
        LinkedNode::new(self.root()).find(span)
    }

    /// Get the byte range for the given span number (and optional sub-range) in
    /// this file.
    ///
    /// The main way to get a [`SpanNumber`] is by unpacking a span with
    /// [`Span::get`], but it's likely easier to use `WorldExt::range` instead.
    pub fn range(
        &self,
        num: SpanNumber,
        sub_range: Option<SubRange>,
    ) -> Option<Range<usize>> {
        let overall = LinkedNode::new(self.root()).find_number(num)?.range();
        if let Some(sub_range) = sub_range {
            let range = sub_range.to_absolute(overall.start);
            assert!(range.end <= overall.end);
            Some(range)
        } else {
            Some(overall)
        }
    }
}

impl Debug for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Source({:?})", self.id().vpath())
    }
}

impl AsRef<str> for Source {
    fn as_ref(&self) -> &str {
        self.text()
    }
}

#[cfg(test)]
mod test {
    use super::Source;
    use crate::{LinkedNode, Side, Span, SubRange};

    #[test]
    fn test_source_sub_ranges() {
        let text = "= head <label>";
        let source = Source::detached(text);
        let get = |span: Span, sub_range| {
            let num = crate::SpanNumber(span.number());
            &text[source.range(num, sub_range).unwrap()]
        };
        let head = LinkedNode::new(source.root()).leaf_at(2, Side::After).unwrap().span();
        assert_eq!(get(head, None), "head");
        assert_eq!(get(head, SubRange::new(1, 3)), "ea");
        assert_eq!(get(head, SubRange::new(0, 1)), "h");
        assert_eq!(get(head, SubRange::new(0, 4)), "head");
        assert_eq!(get(head, SubRange::new(3, 4)), "d");
        let root = source.root().span();
        assert_eq!(get(root, None), text);
        assert_eq!(get(root, SubRange::new(3, 10)), "ead <la");
        assert_eq!(get(root, SubRange::new(0, 10)), "= head <la");
        assert_eq!(get(root, SubRange::new(3, 14)), "ead <label>");
    }
}
