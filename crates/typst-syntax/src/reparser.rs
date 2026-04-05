use std::ops::Range;

use crate::{
    Span, SyntaxKind, SyntaxNode, is_newline, parse, reparse_block, reparse_markup,
};

/// Refresh the given syntax node with as little parsing as possible.
///
/// Takes the new text, the range in the old text that was replaced and the
/// length of the replacement and returns the range in the new text that was
/// ultimately reparsed.
///
/// The high-level API for this function is
/// [`Source::edit`](crate::Source::edit).
pub fn reparse(
    root: &mut SyntaxNode,
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
) -> Range<usize> {
    try_reparse(text, replaced, replacement_len, None, root, 0).unwrap_or_else(|| {
        let id = root.span().id();
        *root = parse(text);
        if let Some(id) = id {
            root.numberize(id, Span::FULL).unwrap();
        }
        0..text.len()
    })
}

/// Try to reparse inside the given node, returning the range that was
/// ultimately reparsed.
///
/// We start by doing a depth-first search for the innermost node or nodes which
/// fully surround the replaced range. This can be a single node that is a
/// code/content block or one or more nodes that are markup expressions and are
/// directly inside a markup block or the top-level markup. In either case, we
/// call the parser and succeed only if the parsed text has balanced delimiters
/// with the same delimiter nesting level as before. Otherwise, we expand the
/// set of markup expressions outwards or return upwards until we either get a
/// parse that does succeed or we parse the entire text.
///
/// Note that we currently only reparse markup expressions at the top-level or
/// directly inside a markup block. E.g. we don't reparse markup expressions
/// inside lists or headings, etc. In the past we did reparse those, but the
/// implementation was very buggy due to edge cases surrounding indentation and
/// newlines, and was eventually removed without much performance impact. It's
/// still potentially desireable to handle some of those cases (individual list
/// items can get quite long in practice), but only if the implementation can be
/// easily reasoned as correct and shows a measured performance improvement.
///
/// We also do not currently reparse math in any capacity, but it would not be
/// too difficult to include equations as another kind of block, or reparse math
/// expressions similarly to markup expressions.
fn try_reparse(
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
    parent_kind: Option<SyntaxKind>,
    node: &mut SyntaxNode,
    offset: usize,
) -> Option<Range<usize>> {
    let (overlap, start_offset) = overlapping_children(node, replaced.clone(), offset)?;

    let node_kind = node.kind();
    let children = node.children_mut();

    if let [child] = &mut children[overlap.clone()]
        && start_offset < replaced.start
        && replaced.end < start_offset + child.len()
        child.is_inner()
    {
        // A single child fully surrounds the edit. We either reparse within the
        // child, or reparse the child itself (if the child is a block).
        let prev_len = child.len();
        let prev_desc = child.descendants();
        let new_len = prev_len + replacement_len - replaced.len();
        let new_range = start_offset..start_offset + new_len;

        // Recursively descend and try to reparse at a lower level.
        if let Some(range) = try_reparse(
            text,
            replaced.clone(),
            replacement_len,
            Some(node_kind),
            child,
            start_offset,
        ) {
            // A lower level reparse succeeded! Update this node and return the
            // reparsed range.
            assert_eq!(child.len(), new_len);
            let new_desc = child.descendants();
            node.update_parent(prev_len, new_len, prev_desc, new_desc);
            return Some(range);
        }

        // This is the innermost block which fully surrounds the text (and
        // hasn't failed at reparsing yet), reparse!
        if child.kind().is_block()
            && let Some(reparsed) = reparse_block(text, new_range.clone())
        {
            // Reparsing succeeded, but we can still fail if we're out of span
            // numbers to assign to nodes (this is rare).
            return node
                .replace_children(overlap, vec![reparsed])
                .is_ok()
                .then_some(new_range);
        }
    }

    if node_kind == SyntaxKind::Markup
        && matches!(parent_kind, None | Some(SyntaxKind::ContentBlock))
    {
        expand_and_reparse_markup(
            text,
            replaced,
            replacement_len,
            node,
            overlap,
            offset,
            parent_kind.is_none(),
        )
    } else {
        None
    }
}

/// Reparse a range of markup expressions, expanding the range exponentially on
/// each iteration.
fn expand_and_reparse_markup(
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
    node: &mut SyntaxNode,
    overlap: Range<usize>,
    offset: usize,
    top_level: bool,
) -> Option<Range<usize>> {
    let children = node.children().as_slice();

    // Reparse a segment. Retries until it works, taking exponentially more
    // children into account.
    let mut expansion = 1;
    loop {
        // Add slack in both directions.
        let mut start = overlap.start.saturating_sub(expansion.max(2));
        let mut end = (overlap.end + expansion).min(children.len());

        // Expand to the left.
        while start > 0 && expand(&children[start]) {
            start -= 1;
        }

        // Expand to the right.
        while end < children.len() && expand(&children[end]) {
            end += 1;
        }

        // Also take hash.
        if start > 0 && children[start - 1].kind() == SyntaxKind::Hash {
            start -= 1;
        }

        // Synthesize what `at_start` and `nesting` would be at the start of the
        // reparse.
        let mut prefix_len = 0;
        let mut nesting = 0;
        let mut at_start = true;
        for child in &children[..start] {
            prefix_len += child.len();
            next_at_start(child, &mut at_start);
            next_nesting(child, &mut nesting);
        }

        // Determine what `at_start` will have to be at the end of the reparse.
        let mut prev_len = 0;
        let mut prev_at_start_after = at_start;
        let mut prev_nesting_after = nesting;
        for child in &children[start..end] {
            prev_len += child.len();
            next_at_start(child, &mut prev_at_start_after);
            next_nesting(child, &mut prev_nesting_after);
        }

        // Determine the range in the new text that we want to reparse.
        let shifted = offset + prefix_len;
        let new_len = prev_len + replacement_len - replaced.len();
        let new_range = shifted..shifted + new_len;
        let at_end = end == children.len();

        // Reparse!
        let reparsed = reparse_markup(
            text,
            new_range.clone(),
            &mut at_start,
            &mut nesting,
            top_level,
        );

        if let Some(newborns) = reparsed {
            // If more children follow, at_start must match its previous value.
            // Similarly, if we children follow or we not top-level the nesting
            // must match its previous value.
            if (at_end || at_start == prev_at_start_after)
                && ((at_end && top_level) || nesting == prev_nesting_after)
            {
                // Reparsing succeeded, but we can still fail if we're out of
                // span numbers to assign to nodes (this is rare).
                return node
                    .replace_children(start..end, newborns)
                    .is_ok()
                    .then_some(new_range);
            }
        }

        // If it didn't even work with all children, we give up.
        if start == 0 && at_end {
            break;
        }

        // Exponential expansion to both sides.
        expansion *= 2;
    }

    None
}

/// The indices and start offset of the children which overlap a replaced range.
/// Returns `None` if the children don't fully cover the range.
fn overlapping_children(
    node: &SyntaxNode,
    range: Range<usize>,
    mut offset: usize,
) -> Option<(Range<usize>, usize)> {
    if !node.is_inner() // Only inner nodes have children.
        || !(offset <= range.start && range.end <= offset + node.len())
    {
        // No set of children will fully cover the range, no need to search.
        return None;
    }
    let mut index = 0;
    let mut start = 0;
    let mut start_offset = offset;
    for child in node.children() {
        if offset < range.start {
            start = index;
            start_offset = offset;
        }
        offset += child.len();
        index += 1;
        if range.end < offset {
            break;
        }
    }
    debug_assert!(start_offset <= range.start);
    debug_assert!(range.end <= offset);
    Some((start..index, start_offset))
}

/// Whether the selection should be expanded beyond a node of this kind.
fn expand(node: &SyntaxNode) -> bool {
    let kind = node.kind();
    kind.is_trivia()
        || kind.is_error()
        || kind == SyntaxKind::Semicolon
        || node.text() == "/"
        || node.text() == ":"
}

/// Whether `at_start` would still be true after this node given the
/// previous value of the property.
fn next_at_start(node: &SyntaxNode, at_start: &mut bool) {
    let kind = node.kind();
    if kind.is_trivia() {
        *at_start |= kind == SyntaxKind::Parbreak
            || (kind == SyntaxKind::Space && node.text().chars().any(is_newline));
    } else {
        *at_start = false;
    }
}

/// Update `nesting` based on the node.
fn next_nesting(node: &SyntaxNode, nesting: &mut usize) {
    if node.kind() == SyntaxKind::Text {
        match node.text().as_str() {
            "[" => *nesting += 1,
            "]" if *nesting > 0 => *nesting -= 1,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use crate::{Source, Span, parse};

    /// How to replace text in the test string.
    enum Edit {
        /// Insert at the end.
        End,
        /// Insert at an index.
        At(usize),
        /// Replace the given range.
        Range(Range<usize>),
        /// Replace the first match in the original.
        Match(&'static str),
        /// Replace at the index after the first match of this string.
        After(&'static str),
    }

    impl Edit {
        #[track_caller]
        fn into_range(self, text: &str) -> Range<usize> {
            match self {
                Self::End => text.len()..text.len(),
                Self::At(index) => {
                    assert!(text.len() >= index, "index is out of bounds");
                    index..index
                }
                Self::Range(range) => {
                    assert!(text.len() >= range.end, "range is out of bounds");
                    range
                }
                Self::Match(pat) => {
                    let start = text.find(pat).expect("pattern must exist in original");
                    start..start + pat.len()
                }
                Self::After(pat) => {
                    let start = text.find(pat).expect("pattern must exist in original");
                    let end = start + pat.len();
                    end..end
                }
            }
        }
    }

    /// What kind of reparsing happened.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Reparse<'a> {
        /// The whole text was reparsed.
        All,
        /// The text was parsed incrementally matching this string.
        Incr(&'a str),
    }

    #[track_caller]
    fn test(text: &str, edit: Edit, with: &str, expected: Reparse) {
        let mut source = Source::detached(text);
        let orig_tree = source.root().clone();
        // `Source::edit()` is the public interface for reparsing.
        let replaced_range = source.edit(edit.into_range(text), with);
        let mut reparsed_tree = source.root().clone();
        let mut normal_parse = parse(source.text());
        reparsed_tree.synthesize(Span::detached());
        normal_parse.synthesize(Span::detached());
        if reparsed_tree != normal_parse {
            eprintln!("Original source text: {text:?}");
            eprintln!("Original tree:\n{orig_tree:#?}");
            eprintln!("New source text: {:?}", source.text());
            eprintln!("Reparsed tree:\n{reparsed_tree:#?}");
            eprintln!("Normal parse tree:\n{normal_parse:#?}");
            panic!("Reparsed tree did not match normal parse");
        }
        let actual = if replaced_range == (0..source.text().len()) {
            Reparse::All
        } else {
            Reparse::Incr(&source.text()[replaced_range])
        };
        assert_eq!(actual, expected);
    }

    /// Basic tests for the reparsing algorithm and the testing framework.
    #[test]
    fn test_reparse_basic() {
        use Reparse::*;
        // Replace everything with something else:
        test("some content", Edit::Match("some content"), "do it", All);
        test("some content", Edit::Range(0..12), "", All);
        test("", Edit::At(0), "do it", All);
        test("", Edit::End, "do it", All);
        // Add something at the end:
        test("some content", Edit::After("some content"), " do it", All);
    }

    /// Test incremental reparsing of markup expressions.
    #[test]
    fn test_reparse_markup() {
        use Reparse::*;
        // Tilde is useful because it always creates a distinct token, whereas
        // spaces may join with adjacent text as one token.
        test("abc~def~gh~", Edit::Range(5..6), "+", Incr("abc~d+f~"));
        test("~~~~~~~", Edit::Range(3..4), "A", Incr("~~~A~~"));
        test("abc~~", Edit::Match("b"), "", Incr("ac~"));
        // Newlines behave slightly differently to tildes:
        test("~b~c~d~e", Edit::At(0), "a", Incr("a~b"));
        test("a~b~c~d~", Edit::End, "e", Incr("~d~e"));
        test("\nb\nc\nd\ne", Edit::At(0), "a", Incr("a\nb\n"));
        test("a\nb\nc\nd\n", Edit::End, "e", Incr("c\nd\ne"));
        test("#var. hello", Edit::Match(" "), " ", All);
        test("#var;hello", Edit::Range(9..10), "a", All);
        test("https:/world", Edit::After("/"), "/", All);
        test("hello  world", Edit::Match("world"), "walkers", All);
        test("a d e", Edit::Match(" d"), " b c d", All);
        test("~*~*~", Edit::At(2), "*", All);
        test("::1\n2. a\n3", Edit::After(" "), "4", Incr("1\n2. 4a\n"));
        test("* #{1+2} *", Edit::Match("2"), "3", Incr("{1+3}"));
        test("#{(0, 1, 2)}", Edit::Match("1"), "11pt", Incr("{(0, 11pt, 2)}"));
        test("\n= A heading", Edit::After("A"), "n evocative", All);
        test("#call() abc~d", Edit::After("()"), "[]", Incr("#call()[] abc"));
        test("a your thing a", Edit::Range(6..7), "a", All);
        test("#grid(columns: (auto, 1fr, 40%))", Edit::Match("auto"), "4pt", All);
        test(
            "abc\n= a head\njoke",
            Edit::Match("\n"),
            "\nmore\n\n",
            Incr("abc\nmore\n\n= a head\n"),
        );
        test("#show f: a => b..", Edit::End, "c", All);
        test("#for", Edit::End, "//", All);
        test("a\n#let \nb", Edit::At(7), "i", Incr("#let i\nb"));
        test("#{{let x = z}; a = 1} b", Edit::At(7), "//", All);
        test("a ```typst hello```", Edit::Range(16..17), "", All);
        test("a{b}c", Edit::At(1), "#", All);
        test("a#{b}c", Edit::Match("#"), "", All);
    }

    /// Test incremental reparsing of code and content blocks.
    #[test]
    fn test_reparse_block() {
        use Reparse::*;
        test("Hello #{ x + 1 }!", Edit::Match("x"), "abc", Incr("{ abc + 1 }"));
        test("A#{}!", Edit::After("{"), "\"", All);
        test("#{ [= x] }!", Edit::After("="), "=", Incr("== x"));
        test("#[[]]", Edit::At(3), "\\", Incr("[[\\]]"));
        test("#[[ab]]", Edit::Match("b"), "\\", Incr("[[a\\]]"));
        test("#{}}", Edit::After("{"), "{", All);
        test("A: #[BC]", Edit::After("B"), "{", Incr("B{C"));
        test("A: #[BC]", Edit::After("B"), "#{", Incr("B#{C"));
        test("A: #[BC]", Edit::After("B"), "#{}", Incr("B#{}C"));
        test("#{\"ab\"}A", Edit::At(5), "c", Incr("{\"abc\"}"));
        test("#{\"ab\"}A", Edit::Range(5..6), "c", All);
        test("a#[]b", Edit::After("["), "#{", Incr("[#{]"));
        test("a#{call(); abc}b", Edit::At(8), "[]", Incr("{call([]); abc}"));
        test(
            "a #while x {\n g(x) \n}  b",
            Edit::After("{"),
            "//",
            Incr("{//\n g(x) \n}"),
        );
        test("a#[]b", Edit::After("["), "[hey]", Incr("[[hey]]"));
    }

    /// Test unclosed strings/blocks in embedded code.
    #[test]
    fn test_reparse_unclosed_embedded() {
        use Reparse::*;
        test("#\"a\nb\nc", Edit::End, "\"", All);
        test("#return \"a\nb\nc", Edit::End, "\"", All);
        test("#import \"@preview/pkg", Edit::End, "\"", All);
        test("#import [a\nb\nc", Edit::End, "]", All);
        test("#import $a\nb\nc", Edit::End, "$", All);
        test("#set thing\"a\n", Edit::End, "\"", All);
        test("#let x = \"a\nb\nc", Edit::End, "\"", All);
        test("#let x = {\"a\nb\nc}", Edit::After("c"), "\"", Incr("{\"a\nb\nc\"}"));
        test("#if x {\"a\nb\nc", Edit::End, "\"", All);
        test("#if x \"a\nb\nc", Edit::End, "\"", All);
        test("#if x \"a\nb\nc", Edit::After("x "), "{}", All);
        test("#if x $a\nb\nc", Edit::End, "$", All);
        test("#for x in \"a\nb\nc", Edit::End, "\"", All);
        test("#for x \"a\nb\nc", Edit::After("x "), "in ", All);
    }
}
