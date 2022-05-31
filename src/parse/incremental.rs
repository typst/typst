use std::ops::Range;
use std::sync::Arc;

use crate::syntax::{InnerNode, NodeKind, SyntaxNode};

use super::{
    is_newline, parse, reparse_code_block, reparse_content_block, reparse_markup_elements,
};

/// Refresh the given syntax node with as little parsing as possible.
///
/// Takes the new source, the range in the old source that was replaced and the
/// length of the replacement.
///
/// Returns the range in the new source that was ultimately reparsed.
pub fn reparse(
    root: &mut SyntaxNode,
    src: &str,
    replaced: Range<usize>,
    replacement_len: usize,
) -> Range<usize> {
    if let SyntaxNode::Inner(inner) = root {
        let reparser = Reparser { src, replaced, replacement_len };
        if let Some(range) = reparser.reparse_step(Arc::make_mut(inner), 0, true) {
            return range;
        }
    }

    *root = parse(src);
    0 .. src.len()
}

/// Allows partial refreshs of the syntax tree.
///
/// This struct holds a description of a change. Its methods can be used to try
/// and apply the change to a syntax tree.
struct Reparser<'a> {
    /// The new source code, with the change applied.
    src: &'a str,
    /// Which range in the old source file was changed.
    replaced: Range<usize>,
    /// How many characters replaced the text in `replaced`.
    replacement_len: usize,
}

impl Reparser<'_> {
    /// Try to reparse inside the given node.
    fn reparse_step(
        &self,
        node: &mut InnerNode,
        mut offset: usize,
        outermost: bool,
    ) -> Option<Range<usize>> {
        let is_markup = matches!(node.kind(), NodeKind::Markup(_));
        let original_count = node.children().len();
        let original_offset = offset;

        let mut search = SearchState::default();
        let mut ahead_nontrivia = None;

        // Whether the first node that should be replaced is at start.
        let mut at_start = true;

        // Whether the last searched child is the outermost child.
        let mut child_outermost = false;

        // Find the the first child in the range of children to reparse.
        for (i, child) in node.children().enumerate() {
            let pos = NodePos { idx: i, offset };
            let child_span = offset .. offset + child.len();

            match search {
                SearchState::NoneFound => {
                    // The edit is contained within the span of the current element.
                    if child_span.contains(&self.replaced.start)
                        && child_span.end >= self.replaced.end
                    {
                        // In Markup mode, we want to consider a non-whitespace
                        // neighbor if the edit is on the node boundary.
                        search = if is_markup && child_span.end == self.replaced.end {
                            SearchState::RequireNonTrivia(pos)
                        } else {
                            SearchState::Contained(pos)
                        };
                    } else if child_span.contains(&self.replaced.start) {
                        search = SearchState::Inside(pos);
                    } else {
                        // We look only for non spaces, non-semicolon and also
                        // reject text that points to the special case for URL
                        // evasion and line comments.
                        if !child.kind().is_space()
                            && child.kind() != &NodeKind::Semicolon
                            && child.kind() != &NodeKind::Text('/'.into())
                            && (ahead_nontrivia.is_none()
                                || self.replaced.start > child_span.end)
                        {
                            ahead_nontrivia = Some((pos, at_start));
                        }
                        at_start = child.kind().is_at_start(at_start);
                    }
                }
                SearchState::Inside(start) => {
                    if child_span.end == self.replaced.end {
                        search = SearchState::RequireNonTrivia(start);
                    } else if child_span.end > self.replaced.end {
                        search = SearchState::SpanFound(start, pos);
                    }
                }
                SearchState::RequireNonTrivia(start) => {
                    if !child.kind().is_trivia() {
                        search = SearchState::SpanFound(start, pos);
                    }
                }
                _ => unreachable!(),
            }

            offset += child.len();
            child_outermost = outermost && i + 1 == original_count;

            if search.done().is_some() {
                break;
            }
        }

        // If we were looking for a non-whitespace element and hit the end of
        // the file here, we instead use EOF as the end of the span.
        if let SearchState::RequireNonTrivia(start) = search {
            search = SearchState::SpanFound(start, NodePos {
                idx: node.children().len() - 1,
                offset: offset - node.children().last().unwrap().len(),
            })
        }

        if let SearchState::Contained(pos) = search {
            let child = &mut node.children_mut()[pos.idx];
            let prev_len = child.len();

            if let Some(range) = match child {
                SyntaxNode::Inner(node) => {
                    self.reparse_step(Arc::make_mut(node), pos.offset, child_outermost)
                }
                SyntaxNode::Leaf(_) => None,
            } {
                let new_len = child.len();
                node.update_parent(new_len, prev_len);
                return Some(range);
            }

            let superseded_span = pos.offset .. pos.offset + prev_len;
            let func: Option<ReparseMode> = match child.kind() {
                NodeKind::CodeBlock => Some(ReparseMode::Code),
                NodeKind::ContentBlock => Some(ReparseMode::Content),
                _ => None,
            };

            // Return if the element was reparsable on its own, otherwise try to
            // treat it as a markup element.
            if let Some(func) = func {
                if let Some(result) = self.replace(
                    node,
                    func,
                    pos.idx .. pos.idx + 1,
                    superseded_span,
                    outermost,
                ) {
                    return Some(result);
                }
            }
        }

        // Save the current indent if this is a markup node and stop otherwise.
        let indent = match node.kind() {
            NodeKind::Markup(n) => *n,
            _ => return None,
        };

        let (mut start, end) = search.done()?;
        if let Some((ahead, ahead_at_start)) = ahead_nontrivia {
            let ahead_kind = node.children().as_slice()[ahead.idx].kind();

            if start.offset == self.replaced.start
                || ahead_kind.only_at_start()
                || !ahead_kind.only_in_markup()
            {
                start = ahead;
                at_start = ahead_at_start;
            }
        } else {
            start = NodePos { idx: 0, offset: original_offset };
        }

        let superseded_span =
            start.offset .. end.offset + node.children().as_slice()[end.idx].len();

        self.replace(
            node,
            ReparseMode::MarkupElements(at_start, indent),
            start.idx .. end.idx + 1,
            superseded_span,
            outermost,
        )
    }

    fn replace(
        &self,
        node: &mut InnerNode,
        mode: ReparseMode,
        superseded_idx: Range<usize>,
        superseded_span: Range<usize>,
        outermost: bool,
    ) -> Option<Range<usize>> {
        let superseded_start = superseded_idx.start;

        let differential: isize =
            self.replacement_len as isize - self.replaced.len() as isize;
        let newborn_end = (superseded_span.end as isize + differential) as usize;
        let newborn_span = superseded_span.start .. newborn_end;

        let mut prefix = "";
        for (i, c) in self.src[.. newborn_span.start].char_indices().rev() {
            if is_newline(c) {
                break;
            }
            prefix = &self.src[i .. newborn_span.start];
        }

        let (newborns, terminated, amount) = match mode {
            ReparseMode::Code => reparse_code_block(
                &prefix,
                &self.src[newborn_span.start ..],
                newborn_span.len(),
            ),
            ReparseMode::Content => reparse_content_block(
                &prefix,
                &self.src[newborn_span.start ..],
                newborn_span.len(),
            ),
            ReparseMode::MarkupElements(at_start, indent) => reparse_markup_elements(
                &prefix,
                &self.src[newborn_span.start ..],
                newborn_span.len(),
                differential,
                &node.children().as_slice()[superseded_start ..],
                at_start,
                indent,
            ),
        }?;

        // Do not accept unclosed nodes if the old node wasn't at the right edge
        // of the tree.
        if !outermost && !terminated {
            return None;
        }

        node.replace_children(superseded_start .. superseded_start + amount, newborns);
        Some(newborn_span)
    }
}

/// The position of a syntax node.
#[derive(Clone, Copy, Debug, PartialEq)]
struct NodePos {
    /// The index in the parent node.
    idx: usize,
    /// The byte offset in the string.
    offset: usize,
}

/// Encodes the state machine of the search for the node which is pending for
/// replacement.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SearchState {
    /// Neither an end nor a start have been found as of now.
    /// The last non-whitespace child is continually saved.
    NoneFound,
    /// The search has concluded by finding a node that fully contains the
    /// modifications.
    Contained(NodePos),
    /// The search has found the start of the modified nodes.
    Inside(NodePos),
    /// The search has found the end of the modified nodes but the change
    /// touched its boundries so another non-trivia node is needed.
    RequireNonTrivia(NodePos),
    /// The search has concluded by finding a start and an end index for nodes
    /// with a pending reparse.
    SpanFound(NodePos, NodePos),
}

impl Default for SearchState {
    fn default() -> Self {
        Self::NoneFound
    }
}

impl SearchState {
    fn done(self) -> Option<(NodePos, NodePos)> {
        match self {
            Self::NoneFound => None,
            Self::Contained(s) => Some((s, s)),
            Self::Inside(_) => None,
            Self::RequireNonTrivia(_) => None,
            Self::SpanFound(s, e) => Some((s, e)),
        }
    }
}

/// Which reparse function to choose for a span of elements.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ReparseMode {
    /// Reparse a code block, including its braces.
    Code,
    /// Reparse a content block, including its square brackets.
    Content,
    /// Reparse elements of the markup. The variant carries whether the node is
    /// `at_start` and the minimum indent of the containing markup node.
    MarkupElements(bool, usize),
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::parse::tests::check;
    use crate::source::SourceFile;

    #[track_caller]
    fn test(prev: &str, range: Range<usize>, with: &str, goal: Range<usize>) {
        let mut source = SourceFile::detached(prev);
        let range = source.edit(range, with);
        check(source.src(), source.root(), &parse(source.src()));
        assert_eq!(range, goal);
    }

    #[test]
    fn test_parse_incremental_simple_replacements() {
        test("hello  world", 7 .. 12, "walkers", 0 .. 14);
        test("some content", 0..12, "", 0..0);
        test("", 0..0, "do it", 0..5);
        test("a d e", 1 .. 3, " b c d", 0 .. 9);
        test("a #f() e", 1 .. 6, " b c d", 0 .. 9);
        test("a\nb\nc\nd\ne\n", 5 .. 5, "c", 2 .. 7);
        test("a\n\nb\n\nc\n\nd\n\ne\n", 7 .. 7, "c", 3 .. 10);
        test("a\nb\nc *hel a b lo* d\nd\ne", 13..13, "c ", 6..20);
        test("~~ {a} ~~", 4 .. 5, "b", 3 .. 6);
        test("{(0, 1, 2)}", 5 .. 6, "11pt", 0..14);
        test("\n= A heading", 3 .. 3, "n evocative", 3 .. 23);
        test("for~your~thing", 9 .. 9, "a", 4 .. 15);
        test("a your thing a", 6 .. 7, "a", 0 .. 14);
        test("{call(); abc}", 7 .. 7, "[]", 0 .. 15);
        test("#call() abc", 7 .. 7, "[]", 0 .. 10);
        test("hi[\n- item\n- item 2\n    - item 3]", 11 .. 11, "  ", 2 .. 35);
        test("hi\n- item\nno item\n    - item 3", 10 .. 10, "- ", 3..19);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 16 .. 20, "none", 0..99);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 33 .. 42, "[_gronk_]", 33..42);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 34 .. 41, "_bar_", 33 .. 40);
        test("{let i=1; for x in range(5) {i}}", 6 .. 6, " ", 0 .. 33);
        test("{let i=1; for x in range(5) {i}}", 13 .. 14, "  ", 0 .. 33);
        test("hello~~{x}", 7 .. 10, "#f()", 5 .. 11);
        test("this~is -- in my opinion -- spectacular", 8 .. 10, "---", 5 .. 25);
        test("understanding `code` is complicated", 15 .. 15, "C ", 14 .. 22);
        test("{ let x = g() }", 10 .. 12, "f(54", 0 .. 17);
        test(r#"a ```typst hello``` b"#, 16 .. 17, "", 2 .. 18);
        test(r#"a ```typst hello```"#, 16 .. 17, "", 2 .. 18);
        test("#for", 4 .. 4, "//", 0 .. 6);
        test("a\n#let \nb", 7 .. 7, "i", 2 .. 9);
        test("a\n#for i \nb", 9 .. 9, "in", 2 .. 12);
        test("a~https://fun/html", 13..14, "n", 2..18);
    }

    #[test]
    fn test_parse_incremental_whitespace_invariants() {
        test("hello \\ world", 7 .. 8, "a ", 0 .. 14);
        test("hello \\ world", 7 .. 8, " a", 0 .. 14);
        test("x = y", 1 .. 1, " + y", 0 .. 6);
        test("x = y", 1 .. 1, " + y\n", 0 .. 7);
        test("abc\n= a heading\njoke", 3 .. 4, "\nmore\n\n", 0 .. 21);
        test("abc\n= a heading\njoke", 3 .. 4, "\nnot ", 0 .. 19);
        test("#let x = (1, 2 + ;~ Five\r\n\r", 20 .. 23, "2.", 18 .. 23);
        test("hey #myfriend", 4 .. 4, "\\", 0 .. 14);
        test("hey  #myfriend", 4 .. 4, "\\", 3 .. 6);
        test("= foo\nbar\n - a\n - b", 6 .. 9, "", 0 .. 11);
        test("= foo\n  bar\n  baz", 6 .. 8, "", 0 .. 9);
        test(" // hi", 1 .. 1, " ", 0 .. 7);
        test("- \nA", 2..3, "", 0..3);
    }

    #[test]
    fn test_parse_incremental_type_invariants() {
        test("a #for x in array {x}", 18 .. 21, "[#x]", 2 .. 22);
        test("a #let x = 1 {5}", 3 .. 6, "if", 2 .. 11);
        test("a {let x = 1 {5}} b", 3 .. 6, "if", 2 .. 16);
        test("#let x = 1 {5}", 4 .. 4, " if", 0 .. 13);
        test("{let x = 1 {5}}", 4 .. 4, " if", 0 .. 18);
        test("a // b c #f()", 3 .. 4, "", 2 .. 12);
        test("{\nf()\n//g(a)\n}", 6 .. 8, "", 0 .. 12);
        test("a{\nf()\n//g(a)\n}b", 7 .. 9, "", 1 .. 13);
        test("a #while x {\n g(x) \n}  b", 11 .. 11, "//", 0 .. 26);
        test("{(1, 2)}", 1 .. 1, "while ", 0 .. 14);
        test("a b c", 1 .. 1, "{[}", 0 .. 8);
    }

    #[test]
    fn test_parse_incremental_wrongly_or_unclosed_things() {
        test(r#"{"hi"}"#, 4 .. 5, "c", 0 .. 6);
        test(r"this \u{abcd}", 8 .. 9, "", 5 .. 12);
        test(r"this \u{abcd} that", 12 .. 13, "", 5 .. 17);
        test(r"{{let x = z}; a = 1} b", 6 .. 6, "//", 0 .. 24);
        test("a b c", 1 .. 1, " /* letters */", 0 .. 19);
        test("a b c", 1 .. 1, " /* letters", 0 .. 16);
        test("{if i==1 {a} else [b]; b()}", 12 .. 12, " /* letters */", 0 .. 41);
        test("{if i==1 {a} else [b]; b()}", 12 .. 12, " /* letters", 0 .. 38);
        test("~~~~", 2 .. 2, "[]", 0 .. 5);
        test("a[]b", 2 .. 2, "{", 1 .. 4);
        test("[hello]", 2 .. 3, "]", 0 .. 7);
        test("{a}", 1 .. 2, "b", 0 .. 3);
        test("{ a; b; c }", 5 .. 6, "[}]", 0 .. 13);
        test("#a()\n~", 3..4, "{}", 0..7);
        test("[]\n~", 1..2, "#if i==0 {true}", 0..18);
    }
}
