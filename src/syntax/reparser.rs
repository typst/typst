use std::ops::Range;

use super::{
    is_newline, parse, reparse_block, reparse_markup, Span, SyntaxKind, SyntaxNode,
};

/// Refresh the given syntax node with as little parsing as possible.
///
/// Takes the new text, the range in the old text that was replaced and the
/// length of the replacement and returns the range in the new text that was
/// ultimately reparsed.
///
/// The high-level API for this function is
/// [`Source::edit`](super::Source::edit).
pub fn reparse(
    root: &mut SyntaxNode,
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
) -> Range<usize> {
    try_reparse(text, replaced, replacement_len, None, root, 0).unwrap_or_else(|| {
        let id = root.span().source();
        *root = parse(text);
        root.numberize(id, Span::FULL).unwrap();
        0..text.len()
    })
}

/// Try to reparse inside the given node.
fn try_reparse(
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
    parent_kind: Option<SyntaxKind>,
    node: &mut SyntaxNode,
    offset: usize,
) -> Option<Range<usize>> {
    // The range of children which overlap with the edit.
    let mut overlap = usize::MAX..0;
    let mut cursor = offset;
    let node_kind = node.kind();

    for (i, child) in node.children_mut().iter_mut().enumerate() {
        let prev_range = cursor..cursor + child.len();
        let prev_len = child.len();
        let prev_desc = child.descendants();

        // Does the child surround the edit?
        // If so, try to reparse within it or itself.
        if !child.is_leaf() && includes(&prev_range, &replaced) {
            let new_len = prev_len + replacement_len - replaced.len();
            let new_range = cursor..cursor + new_len;

            // Try to reparse within the child.
            if let Some(range) = try_reparse(
                text,
                replaced.clone(),
                replacement_len,
                Some(node_kind),
                child,
                cursor,
            ) {
                assert_eq!(child.len(), new_len);
                let new_desc = child.descendants();
                node.update_parent(prev_len, new_len, prev_desc, new_desc);
                return Some(range);
            }

            // If the child is a block, try to reparse the block.
            if child.kind().is_block() {
                if let Some(newborn) = reparse_block(text, new_range.clone()) {
                    return node
                        .replace_children(i..i + 1, vec![newborn])
                        .is_ok()
                        .then(|| new_range);
                }
            }
        }

        // Does the child overlap with the edit?
        if overlaps(&prev_range, &replaced) {
            overlap.start = overlap.start.min(i);
            overlap.end = i + 1;
        }

        // Is the child beyond the edit?
        if replaced.end < cursor {
            break;
        }

        cursor += child.len();
    }

    // Try to reparse a range of markup expressions within markup. This is only
    // possible if the markup is top-level or contained in a block, not if it is
    // contained in things like headings or lists because too much can go wrong
    // with indent and line breaks.
    if node.kind() == SyntaxKind::Markup
        && (parent_kind.is_none() || parent_kind == Some(SyntaxKind::ContentBlock))
        && !overlap.is_empty()
    {
        // Add one node of slack in both directions.
        let children = node.children_mut();
        let mut start = overlap.start.saturating_sub(1);
        let mut end = (overlap.end + 1).min(children.len());

        // Expand to the left.
        while start > 0 && expand(&children[start]) {
            start -= 1;
        }

        // Expand to the right.
        while end < children.len() && expand(&children[end]) {
            end += 1;
        }

        // Synthesize what `at_start` would be at the start of the reparse.
        let mut prefix_len = 0;
        let mut at_start = true;
        for child in &children[..start] {
            prefix_len += child.len();
            next_at_start(child, &mut at_start);
        }

        // Determine what `at_start` will have to be at the end of the reparse.
        let mut prev_len = 0;
        let mut prev_at_start_after = at_start;
        for child in &children[start..end] {
            prev_len += child.len();
            next_at_start(child, &mut prev_at_start_after);
        }

        let shifted = offset + prefix_len;
        let new_len = prev_len + replacement_len - replaced.len();
        let new_range = shifted..shifted + new_len;
        let stop_kind = match parent_kind {
            Some(_) => SyntaxKind::RightBracket,
            None => SyntaxKind::Eof,
        };

        if let Some(newborns) =
            reparse_markup(text, new_range.clone(), &mut at_start, |kind| {
                kind == stop_kind
            })
        {
            if at_start == prev_at_start_after {
                return node
                    .replace_children(start..end, newborns)
                    .is_ok()
                    .then(|| new_range);
            }
        }
    }

    None
}

/// Whether the inner range is fully contained in the outer one (no touching).
fn includes(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start < inner.start && outer.end > inner.end
}

/// Whether the first and second range overlap or touch.
fn overlaps(first: &Range<usize>, second: &Range<usize>) -> bool {
    (first.start <= second.start && second.start <= first.end)
        || (second.start <= first.start && first.start <= second.end)
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
    if node.kind().is_trivia() {
        if node.text().chars().any(is_newline) {
            *at_start = true;
        }
    } else {
        *at_start = false;
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::super::{parse, Source};

    #[track_caller]
    fn test(prev: &str, range: Range<usize>, with: &str, incremental: bool) {
        let mut source = Source::detached(prev);
        let prev = source.root().clone();
        let range = source.edit(range, with);
        let found = source.root();
        let expected = parse(source.text());
        if found != &expected {
            eprintln!("source:   {:?}", source.text());
            eprintln!("previous: {prev:#?}");
            eprintln!("expected: {expected:#?}");
            eprintln!("found:    {found:#?}");
            panic!("test failed");
        }
        if incremental {
            assert_ne!(source.len_bytes(), range.len());
        } else {
            assert_eq!(source.len_bytes(), range.len());
        }
    }

    #[test]
    fn test_reparse_markup() {
        test("abc~def~ghi", 5..6, "+", true);
        test("~~~~~~~", 3..4, "A", true);
        test("abc~~", 1..2, "", true);
        test("#var;hello", 9..10, "a", false);
        test("https:/world", 7..7, "/", false);
        test("hello  world", 7..12, "walkers", false);
        test("some content", 0..12, "", false);
        test("", 0..0, "do it", false);
        test("a d e", 1..3, " b c d", false);
        test("~*~*~", 2..2, "*", false);
        test("::1\n2. a\n3", 7..7, "4", true);
        test("* {1+2} *", 5..6, "3", true);
        test("{(0, 1, 2)}", 5..6, "11pt", false);
        test("\n= A heading", 4..4, "n evocative", false);
        test("#call() abc~d", 7..7, "[]", true);
        test("a your thing a", 6..7, "a", false);
        test("#grid(columns: (auto, 1fr, 40%))", 16..20, "4pt", false);
        test("abc\n= a heading\njoke", 3..4, "\nmore\n\n", true);
        test("#show f: a => b..", 16..16, "c", false);
        test("#for", 4..4, "//", false);
        test("a\n#let \nb", 7..7, "i", true);
        test("#let x = (1, 2 + ;~ Five\r\n\r", 20..23, "2.", true);
        test(r"{{let x = z}; a = 1} b", 6..6, "//", false);
        test(r#"a ```typst hello```"#, 16..17, "", false);
    }

    #[test]
    fn test_reparse_block() {
        test("Hello { x + 1 }!", 8..9, "abc", true);
        test("A{}!", 2..2, "\"", false);
        test("{ [= x] }!", 4..4, "=", true);
        test("[[]]", 2..2, "\\", false);
        test("[[ab]]", 3..4, "\\", false);
        test("{}}", 1..1, "{", false);
        test("A: [BC]", 5..5, "{", false);
        test("A: [BC]", 5..5, "{}", true);
        test("{\"ab\"}A", 4..4, "c", true);
        test("{\"ab\"}A", 4..5, "c", false);
        test("a[]b", 2..2, "{", false);
        test("a{call(); abc}b", 7..7, "[]", true);
        test("a #while x {\n g(x) \n}  b", 12..12, "//", true);
    }
}
