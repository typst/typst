use std::ops::Range;
use std::sync::Arc;

use crate::syntax::{InnerNode, NodeKind, Span, SyntaxNode};

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
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
) -> Range<usize> {
    if let SyntaxNode::Inner(inner) = root {
        let change = Change { text, replaced, replacement_len };
        if let Some(range) = try_reparse(&change, Arc::make_mut(inner), 0, true, true) {
            return range;
        }
    }

    let id = root.span().source();
    *root = parse(text);
    root.numberize(id, Span::FULL).unwrap();
    0 .. text.len()
}

/// Try to reparse inside the given node.
fn try_reparse(
    change: &Change,
    node: &mut InnerNode,
    mut offset: usize,
    outermost: bool,
    safe_to_replace: bool,
) -> Option<Range<usize>> {
    let is_markup = matches!(node.kind(), NodeKind::Markup { .. });
    let original_count = node.children().len();
    let original_offset = offset;

    let mut search = SearchState::default();
    let mut ahead: Option<Ahead> = None;

    // Whether the first node that should be replaced is at start.
    let mut at_start = true;

    // Whether the last searched child is the outermost child.
    let mut child_outermost = false;

    // Find the the first child in the range of children to reparse.
    for (i, child) in node.children().enumerate() {
        let pos = NodePos { idx: i, offset };
        let child_span = offset .. offset + child.len();
        child_outermost = outermost && i + 1 == original_count;

        match search {
            SearchState::NoneFound => {
                // The edit is contained within the span of the current element.
                if child_span.contains(&change.replaced.start)
                    && child_span.end >= change.replaced.end
                {
                    // In Markup mode, we want to consider a non-whitespace
                    // neighbor if the edit is on the node boundary.
                    search = if is_markup && child_span.end == change.replaced.end {
                        SearchState::RequireNonTrivia(pos)
                    } else {
                        SearchState::Contained(pos)
                    };
                } else if child_span.contains(&change.replaced.start) {
                    search = SearchState::Inside(pos);
                } else if child_span.end == change.replaced.start
                    && change.replaced.start == change.replaced.end
                    && child_outermost
                {
                    search = SearchState::SpanFound(pos, pos);
                } else {
                    // Update compulsary state of `ahead_nontrivia`.
                    if let Some(ahead_nontrivia) = ahead.as_mut() {
                        if let NodeKind::Space { newlines: (1 ..) } = child.kind() {
                            ahead_nontrivia.newline();
                        }
                    }

                    // We look only for non spaces, non-semicolon and also
                    // reject text that points to the special case for URL
                    // evasion and line comments.
                    if !child.kind().is_space()
                        && child.kind() != &NodeKind::Semicolon
                        && child.kind() != &NodeKind::Text('/'.into())
                        && (ahead.is_none() || change.replaced.start > child_span.end)
                        && !ahead.map_or(false, Ahead::is_compulsory)
                    {
                        ahead = Some(Ahead::new(pos, at_start, is_bounded(child.kind())));
                    }

                    at_start = next_at_start(child.kind(), at_start);
                }
            }
            SearchState::Inside(start) => {
                if child_span.end == change.replaced.end {
                    search = SearchState::RequireNonTrivia(start);
                } else if child_span.end > change.replaced.end {
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
        // Do not allow replacement of elements inside of constructs whose
        // opening and closing brackets look the same.
        let safe_inside = is_bounded(node.kind());
        let child = &mut node.children_mut()[pos.idx];
        let prev_len = child.len();
        let prev_descendants = child.descendants();

        if let Some(range) = match child {
            SyntaxNode::Inner(node) => try_reparse(
                change,
                Arc::make_mut(node),
                pos.offset,
                child_outermost,
                safe_inside,
            ),
            SyntaxNode::Leaf(_) => None,
        } {
            let new_len = child.len();
            let new_descendants = child.descendants();
            node.update_parent(prev_len, new_len, prev_descendants, new_descendants);
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
            if let Some(result) = replace(
                change,
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

    // Make sure this is a markup node and that we may replace. If so, save
    // the current indent.
    let min_indent = match node.kind() {
        NodeKind::Markup { min_indent } if safe_to_replace => *min_indent,
        _ => return None,
    };

    let (mut start, end) = search.done()?;
    if let Some(ahead) = ahead {
        if start.offset == change.replaced.start || ahead.is_compulsory() {
            start = ahead.pos;
            at_start = ahead.at_start;
        }
    } else {
        start = NodePos { idx: 0, offset: original_offset };
    }

    let superseded_span =
        start.offset .. end.offset + node.children().as_slice()[end.idx].len();

    replace(
        change,
        node,
        ReparseMode::MarkupElements { at_start, min_indent },
        start.idx .. end.idx + 1,
        superseded_span,
        outermost,
    )
}

/// Reparse the superseded nodes and replace them.
fn replace(
    change: &Change,
    node: &mut InnerNode,
    mode: ReparseMode,
    superseded_idx: Range<usize>,
    superseded_span: Range<usize>,
    outermost: bool,
) -> Option<Range<usize>> {
    let superseded_start = superseded_idx.start;

    let differential: isize =
        change.replacement_len as isize - change.replaced.len() as isize;
    let newborn_end = (superseded_span.end as isize + differential) as usize;
    let newborn_span = superseded_span.start .. newborn_end;

    let mut prefix = "";
    for (i, c) in change.text[.. newborn_span.start].char_indices().rev() {
        if is_newline(c) {
            break;
        }
        prefix = &change.text[i .. newborn_span.start];
    }

    let (newborns, terminated, amount) = match mode {
        ReparseMode::Code => reparse_code_block(
            &prefix,
            &change.text[newborn_span.start ..],
            newborn_span.len(),
        ),
        ReparseMode::Content => reparse_content_block(
            &prefix,
            &change.text[newborn_span.start ..],
            newborn_span.len(),
        ),
        ReparseMode::MarkupElements { at_start, min_indent } => reparse_markup_elements(
            &prefix,
            &change.text[newborn_span.start ..],
            newborn_span.len(),
            differential,
            &node.children().as_slice()[superseded_start ..],
            at_start,
            min_indent,
        ),
    }?;

    // Do not accept unclosed nodes if the old node wasn't at the right edge
    // of the tree.
    if !outermost && !terminated {
        return None;
    }

    node.replace_children(superseded_start .. superseded_start + amount, newborns)
        .ok()?;

    Some(newborn_span)
}

/// A description of a change.
struct Change<'a> {
    /// The new source code, with the change applied.
    text: &'a str,
    /// Which range in the old source file was changed.
    replaced: Range<usize>,
    /// How many characters replaced the text in `replaced`.
    replacement_len: usize,
}

/// Encodes the state machine of the search for the nodes are pending for
/// replacement.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SearchState {
    /// Neither an end nor a start have been found as of now.
    /// The latest non-trivia child is continually saved.
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

/// The position of a syntax node.
#[derive(Clone, Copy, Debug, PartialEq)]
struct NodePos {
    /// The index in the parent node.
    idx: usize,
    /// The byte offset in the string.
    offset: usize,
}

/// An ahead node with an index and whether it is `at_start`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ahead {
    /// The position of the node.
    pos: NodePos,
    /// The `at_start` before this node.
    at_start: bool,
    /// The kind of ahead node.
    kind: AheadKind,
}

/// The kind of ahead node.
#[derive(Clone, Copy, Debug, PartialEq)]
enum AheadKind {
    /// A normal non-trivia child has been found.
    Normal,
    /// An unbounded child has been found. The boolean indicates whether it was
    /// on the current line, in which case adding it to the reparsing range is
    /// compulsory.
    Unbounded(bool),
}

impl Ahead {
    fn new(pos: NodePos, at_start: bool, bounded: bool) -> Self {
        Self {
            pos,
            at_start,
            kind: if bounded {
                AheadKind::Normal
            } else {
                AheadKind::Unbounded(true)
            },
        }
    }

    fn newline(&mut self) {
        if let AheadKind::Unbounded(current_line) = &mut self.kind {
            *current_line = false;
        }
    }

    fn is_compulsory(self) -> bool {
        matches!(self.kind, AheadKind::Unbounded(true))
    }
}

/// Which reparse function to choose for a span of elements.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ReparseMode {
    /// Reparse a code block, including its braces.
    Code,
    /// Reparse a content block, including its square brackets.
    Content,
    /// Reparse elements of the markup. Also specified the initial `at_start`
    /// state for the reparse and the minimum indent of the reparsed nodes.
    MarkupElements { at_start: bool, min_indent: usize },
}

/// Whether changes _inside_ this node are safely encapsulated, so that only
/// this node must be reparsed.
fn is_bounded(kind: &NodeKind) -> bool {
    match kind {
        NodeKind::CodeBlock
        | NodeKind::ContentBlock
        | NodeKind::Backslash
        | NodeKind::Tilde
        | NodeKind::HyphQuest
        | NodeKind::Hyph2
        | NodeKind::Hyph3
        | NodeKind::Dot3
        | NodeKind::Quote { .. }
        | NodeKind::BlockComment
        | NodeKind::Space { .. }
        | NodeKind::Escape(_) => true,
        _ => false,
    }
}

/// Whether `at_start` would still be true after this node given the
/// previous value of the property.
fn next_at_start(kind: &NodeKind, prev: bool) -> bool {
    match kind {
        NodeKind::Space { newlines: (1 ..) } => true,
        NodeKind::Space { .. } | NodeKind::LineComment | NodeKind::BlockComment => prev,
        _ => false,
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::parse::tests::check;
    use crate::source::Source;

    #[track_caller]
    fn test(prev: &str, range: Range<usize>, with: &str, goal: Range<usize>) {
        let mut source = Source::detached(prev);
        let range = source.edit(range, with);
        check(source.text(), source.root(), &parse(source.text()));
        assert_eq!(range, goal);
    }

    #[test]
    fn test_parse_incremental_simple_replacements() {
        test("hello  world", 7 .. 12, "walkers", 0 .. 14);
        test("some content", 0..12, "", 0..0);
        test("", 0..0, "do it", 0..5);
        test("a d e", 1 .. 3, " b c d", 0 .. 9);
        test("*~ *", 2..2, "*", 0..5);
        test("_1_\n2a\n3", 5..5, "4", 4..7);
        test("_1_\n2a\n3~", 8..8, "4", 4..10);
        test("_1_ 2 3a\n4", 7..7, "5", 0..9);
        test("* {1+2} *", 5..6, "3", 2..7);
        test("a #f() e", 1 .. 6, " b c d", 0 .. 9);
        test("a\nb\nc\nd\ne\n", 5 .. 5, "c", 2 .. 7);
        test("a\n\nb\n\nc\n\nd\n\ne\n", 7 .. 7, "c", 3 .. 10);
        test("a\nb\nc *hel a b lo* d\nd\ne", 13..13, "c ", 4..20);
        test("~~ {a} ~~", 4 .. 5, "b", 3 .. 6);
        test("{(0, 1, 2)}", 5 .. 6, "11pt", 0..14);
        test("\n= A heading", 4 .. 4, "n evocative", 0 .. 23);
        test("for~your~thing", 9 .. 9, "a", 0 .. 15);
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
        test("hello~~{x}", 7 .. 10, "#f()", 0 .. 11);
        test("this~is -- in my opinion -- spectacular", 8 .. 10, "---", 0 .. 25);
        test("understanding `code` is complicated", 15 .. 15, "C ", 0 .. 22);
        test("{ let x = g() }", 10 .. 12, "f(54", 0 .. 17);
        test(r#"a ```typst hello``` b"#, 16 .. 17, "", 0 .. 18);
        test(r#"a ```typst hello```"#, 16 .. 17, "", 0 .. 18);
        test("#for", 4 .. 4, "//", 0 .. 6);
        test("#show a: f as b..", 16..16, "c", 0..18);
        test("a\n#let \nb", 7 .. 7, "i", 2 .. 9);
        test("a\n#for i \nb", 9 .. 9, "in", 2 .. 12);
        test("a~https://fun/html", 13..14, "n", 0..18);
    }

    #[test]
    fn test_parse_incremental_whitespace_invariants() {
        test("hello \\ world", 7 .. 8, "a ", 0 .. 14);
        test("hello \\ world", 7 .. 8, " a", 0 .. 14);
        test("x = y", 1 .. 1, " + y", 0 .. 6);
        test("x = y", 1 .. 1, " + y\n", 0 .. 7);
        test("abc\n= a heading\njoke", 3 .. 4, "\nmore\n\n", 0 .. 21);
        test("abc\n= a heading\njoke", 3 .. 4, "\nnot ", 0 .. 19);
        test("#let x = (1, 2 + ;~ Five\r\n\r", 20 .. 23, "2.", 0 .. 23);
        test("hey #myfriend", 4 .. 4, "\\", 0 .. 14);
        test("hey  #myfriend", 4 .. 4, "\\", 0 .. 6);
        test("= foo\nbar\n - a\n - b", 6 .. 9, "", 0 .. 11);
        test("= foo\n  bar\n  baz", 6 .. 8, "", 0 .. 9);
        test(" // hi", 1 .. 1, " ", 0 .. 7);
        test("- \nA", 2..3, "", 0..3);
    }

    #[test]
    fn test_parse_incremental_type_invariants() {
        test("a #for x in array {x}", 18 .. 21, "[#x]", 0 .. 22);
        test("a #let x = 1 {5}", 3 .. 6, "if", 0 .. 11);
        test("a {let x = 1 {5}} b", 3 .. 6, "if", 2 .. 16);
        test("#let x = 1 {5}", 4 .. 4, " if", 0 .. 13);
        test("{let x = 1 {5}}", 4 .. 4, " if", 0 .. 18);
        test("a // b c #f()", 3 .. 4, "", 0 .. 12);
        test("{\nf()\n//g(a)\n}", 6 .. 8, "", 0 .. 12);
        test("a{\nf()\n//g(a)\n}b", 7 .. 9, "", 1 .. 13);
        test("a #while x {\n g(x) \n}  b", 11 .. 11, "//", 0 .. 26);
        test("{(1, 2)}", 1 .. 1, "while ", 0 .. 14);
        test("a b c", 1 .. 1, "{[}", 0 .. 8);
    }

    #[test]
    fn test_parse_incremental_wrongly_or_unclosed_things() {
        test(r#"{"hi"}"#, 4 .. 5, "c", 0 .. 6);
        test(r"this \u{abcd}", 8 .. 9, "", 0 .. 12);
        test(r"this \u{abcd} that", 12 .. 13, "", 0 .. 17);
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
