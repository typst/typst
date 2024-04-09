use std::ops::Range;

use crate::{
    is_newline, parse, reparse_block, reparse_markup, Span, SyntaxKind, SyntaxNode,
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
    #[allow(clippy::reversed_empty_ranges)]
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
                        .then_some(new_range);
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
    if overlap.is_empty()
        || node.kind() != SyntaxKind::Markup
        || !matches!(parent_kind, None | Some(SyntaxKind::ContentBlock))
    {
        return None;
    }

    let children = node.children_mut();

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

        // Stop parsing early if this kind is encountered.
        let stop_kind = match parent_kind {
            Some(_) => SyntaxKind::RightBracket,
            None => SyntaxKind::End,
        };

        // Reparse!
        let reparsed = reparse_markup(
            text,
            new_range.clone(),
            &mut at_start,
            &mut nesting,
            |kind| kind == stop_kind,
        );

        if let Some(newborns) = reparsed {
            // If more children follow, at_start must match its previous value.
            // Similarly, if we children follow or we not top-level the nesting
            // must match its previous value.
            if (at_end || at_start == prev_at_start_after)
                && ((at_end && parent_kind.is_none()) || nesting == prev_nesting_after)
            {
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

    use crate::{parse, Source, Span};

    #[track_caller]
    fn test(prev: &str, range: Range<usize>, with: &str, incremental: bool) {
        let mut source = Source::detached(prev);
        let prev = source.root().clone();
        let range = source.edit(range, with);
        let mut found = source.root().clone();
        let mut expected = parse(source.text());
        found.synthesize(Span::detached());
        expected.synthesize(Span::detached());
        if found != expected {
            eprintln!("source:   {:?}", source.text());
            eprintln!("previous: {prev:#?}");
            eprintln!("expected: {expected:#?}");
            eprintln!("found:    {found:#?}");
            panic!("test failed");
        }
        if incremental {
            assert_ne!(source.len_bytes(), range.len(), "should have been incremental");
        } else {
            assert_eq!(
                source.len_bytes(),
                range.len(),
                "shouldn't have been incremental"
            );
        }
    }

    #[test]
    fn test_reparse_markup() {
        test("abc~def~gh~", 5..6, "+", true);
        test("~~~~~~~", 3..4, "A", true);
        test("abc~~", 1..2, "", true);
        test("#var. hello", 5..6, " ", false);
        test("#var;hello", 9..10, "a", false);
        test("https:/world", 7..7, "/", false);
        test("hello  world", 7..12, "walkers", false);
        test("some content", 0..12, "", false);
        test("", 0..0, "do it", false);
        test("a d e", 1..3, " b c d", false);
        test("~*~*~", 2..2, "*", false);
        test("::1\n2. a\n3", 7..7, "4", true);
        test("* #{1+2} *", 6..7, "3", true);
        test("#{(0, 1, 2)}", 6..7, "11pt", true);
        test("\n= A heading", 4..4, "n evocative", false);
        test("#call() abc~d", 7..7, "[]", true);
        test("a your thing a", 6..7, "a", false);
        test("#grid(columns: (auto, 1fr, 40%))", 16..20, "4pt", false);
        test("abc\n= a heading\njoke", 3..4, "\nmore\n\n", true);
        test("#show f: a => b..", 16..16, "c", false);
        test("#for", 4..4, "//", false);
        test("a\n#let \nb", 7..7, "i", true);
        test(r"#{{let x = z}; a = 1} b", 7..7, "//", false);
        test(r#"a ```typst hello```"#, 16..17, "", false);
        test("a{b}c", 1..1, "#", false);
        test("a#{b}c", 1..2, "", false);
    }

    #[test]
    fn test_reparse_block() {
        test("Hello #{ x + 1 }!", 9..10, "abc", true);
        test("A#{}!", 3..3, "\"", false);
        test("#{ [= x] }!", 5..5, "=", true);
        test("#[[]]", 3..3, "\\", true);
        test("#[[ab]]", 4..5, "\\", true);
        test("#{}}", 2..2, "{", false);
        test("A: #[BC]", 6..6, "{", true);
        test("A: #[BC]", 6..6, "#{", true);
        test("A: #[BC]", 6..6, "#{}", true);
        test("#{\"ab\"}A", 5..5, "c", true);
        test("#{\"ab\"}A", 5..6, "c", false);
        test("a#[]b", 3..3, "#{", true);
        test("a#{call(); abc}b", 8..8, "[]", true);
        test("a #while x {\n g(x) \n}  b", 12..12, "//", true);
        test("a#[]b", 3..3, "[hey]", true);
    }
}
