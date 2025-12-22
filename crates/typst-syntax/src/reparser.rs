use std::ops::Range;

use crate::node::{InnerNode, NumberingResult, Unnumberable};
use crate::{
    Diagnosis, PreferredCompilerVersion, Span, SyntaxKind, SyntaxNode, parse,
    reparse_block, reparse_markup,
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
    preferred_version: PreferredCompilerVersion,
) -> Range<usize> {
    let root_id = root.span().id();

    let mut renumber = true;
    let mut reparsed_range = 0..text.len();
    if let Some(inner) = root.reparse_inner_node_mut()
        && let Some((range, num_result)) = try_reparse_and_renumber(
            text,
            replaced,
            replacement_len,
            None,
            inner,
            0,
            preferred_version,
        )
    {
        reparsed_range = range;
        renumber = num_result.is_err();
    } else {
        *root = parse(text, preferred_version);
    }

    if renumber && let Some(id) = root_id {
        root.numberize(id, Span::FULL).unwrap();
    }

    reparsed_range
}

/// Try to reparse inside the given node, returning the range of new text that
/// was ultimately reparsed.
///
/// If reparsing succeeds, also tries to assign span numbers to the new nodes,
/// returning a [`NumberingResult`]. If the numbering result is an error, i.e.
/// we're out of span numbers at this level, the caller is in charge of
/// assigning span numbers (this should be rare).
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
/// still potentially desirable to handle some of those cases (individual list
/// items can get quite long in practice), but only if the implementation can be
/// easily reasoned as correct and shows a measured performance improvement.
///
/// We also do not currently reparse math in any capacity, but it would not be
/// too difficult to include equations as another kind of block, or reparse math
/// expressions similarly to markup expressions.
fn try_reparse_and_renumber(
    text: &str,
    replaced: Range<usize>,
    replacement_len: usize,
    parent_kind: Option<SyntaxKind>,
    (node, node_kind, node_span): (&mut InnerNode, SyntaxKind, Span),
    offset: usize,
    preferred_version: PreferredCompilerVersion,
) -> Option<(Range<usize>, NumberingResult)> {
    let (overlap, start_offset) = overlapping_children(node, replaced.clone(), offset)?;

    if let [child_node] = &mut node.children[overlap.clone()]
        && let Some((child, child_kind, child_span)) = child_node.reparse_inner_node_mut()
        && start_offset < replaced.start
        && replaced.end < start_offset + child.len
    {
        // A single child fully surrounds the edit. We either reparse within the
        // child, or reparse the child itself (if the child is a block).
        let prev_len = child.len;
        let prev_desc = child.descendants;
        let prev_diag = child.diagnosis;
        let new_len = prev_len + replacement_len - replaced.len();
        let new_range = start_offset..start_offset + new_len;

        // Recursively descend and try to reparse at a lower level.
        if let Some((new_range, mut num_result)) = try_reparse_and_renumber(
            text,
            replaced.clone(),
            replacement_len,
            Some(node_kind),
            (child, child_kind, child_span),
            start_offset,
            preferred_version,
        ) {
            // A lower level reparse succeeded! Update this node and return the
            // reparsed range.
            assert_eq!(child.len, new_len);
            let new_desc = child.descendants;
            let new_diag = child.diagnosis;
            node.update_parent(
                prev_len, new_len, prev_desc, new_desc, prev_diag, new_diag,
            );
            if num_result.is_err() {
                num_result = node.renumber_children(node_span, overlap);
            }
            return Some((new_range, num_result));
        }

        // The child is the innermost block which fully surrounds the text (and
        // hasn't failed at reparsing yet), reparse!
        if child_kind.is_block()
            && let Some(new_child) =
                reparse_block(text, new_range.clone(), preferred_version)
        {
            let new_desc = new_child.descendants();
            let new_diag = new_child.diagnosis();
            *child_node = new_child;
            node.update_parent(
                prev_len, new_len, prev_desc, new_desc, prev_diag, new_diag,
            );
            // Reparsing succeeded! Now try to assign span numbers to the nodes.
            let num_result = node.renumber_children(node_span, overlap);
            return Some((new_range, num_result));
        }
    }

    if node_kind == SyntaxKind::Markup
        && matches!(parent_kind, None | Some(SyntaxKind::ContentBlock))
        && let Some(MarkupReparse { newborns, prev_children, new_range }) =
            expand_and_reparse_markup(
                text,
                preferred_version,
                replaced,
                replacement_len,
                &node.children,
                overlap,
                offset,
                parent_kind.is_none(),
            )
    {
        let reparsed_children = node.replace_children(prev_children, newborns);
        // Reparsing succeeded! Now try to assign span numbers to the nodes.
        let num_result = node.renumber_children(node_span, reparsed_children);
        Some((new_range, num_result))
    } else {
        None
    }
}

/// Reparsed markup expressions. We use a struct to avoid ambiguity since the
/// two [`Range<usize>`] values have different meanings.
struct MarkupReparse {
    /// The newly parsed expressions.
    newborns: Vec<SyntaxNode>,
    /// The range of children that were replaced.
    prev_children: Range<usize>,
    /// The range of new text that was reparsed.
    new_range: Range<usize>,
}

////// Reparse a range of markup expressions, expanding the range exponentially on
/// each iteration until reparsing succeeds.
#[expect(clippy::too_many_arguments)]
fn expand_and_reparse_markup(
    text: &str,
    preferred_version: PreferredCompilerVersion,
    replaced: Range<usize>,
    replacement_len: usize,
    children: &[SyntaxNode],
    overlap: Range<usize>,
    offset: usize,
    top_level: bool,
) -> Option<MarkupReparse> {
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
            preferred_version,
            new_range.clone(),
            &mut at_start,
            &mut nesting,
            top_level,
        );

        if let Some(newborns) = reparsed {
            // Generally, `at_start` and `nesting` need to match their previous
            // values or else the reparse could affect adjacent/surrounding
            // nodes. For example, `= Head` is only a heading at the start of a
            // line, not in the middle of some text. And the outer content block
            // in `#[a[b]c]` would group differently if the first `[` were
            // deleted.
            //
            // But they can safely differ from their previous values under the
            // right conditions:
            // - `at_start` can differ if we're at the end of this markup node,
            //   since no following markup nodes could be affected.
            // - `nesting` can differ if we're _both_ at the end of this markup
            //   node and are reparsing the top-level markup node (i.e. we're
            //   inserting at the very end of a file), since only the top-level
            //   markup node has no surrounding blocks.
            if (at_end || at_start == prev_at_start_after)
                && ((at_end && top_level) || nesting == prev_nesting_after)
            {
                let prev_children = start..end;
                return Some(MarkupReparse { newborns, prev_children, new_range });
            }
        }

        // If it didn't even work with all children, we give up.
        if start == 0 && at_end {
            return None;
        }

        // Exponential expansion to both sides.
        expansion *= 2;
    }
}

/// The indices and start offset of the children which overlap a replaced range.
/// Returns `None` if the children don't fully cover the range. Will never
/// return an empty range if it returns `Some`.
///
/// Note that the returned `Range<usize>` is the range of children in the
/// parent, but the returned `usize` is a byte offset in the source text.
fn overlapping_children(
    node: &InnerNode,
    range: Range<usize>,
    mut offset: usize,
) -> Option<(Range<usize>, usize)> {
    if offset > range.start || range.end > offset + node.len {
        // No set of children will fully cover the range, no need to search.
        return None;
    }
    let mut index = 0;
    let mut start = 0;
    let mut start_offset = offset;
    for child in &node.children {
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
        || node.leaf_text() == "/"
        || node.leaf_text() == ":"
}

/// Whether `at_start` would still be true after this node given the
/// previous value of the property.
fn next_at_start(node: &SyntaxNode, at_start: &mut bool) {
    let kind = node.kind();
    if kind.is_trivia() {
        *at_start |= matches!(kind, SyntaxKind::Parbreak | SyntaxKind::SpaceWithNewline);
    } else {
        *at_start = false;
    }
}

/// Update `nesting` based on the node.
fn next_nesting(node: &SyntaxNode, nesting: &mut usize) {
    if node.kind() == SyntaxKind::Text {
        match node.leaf_text().as_str() {
            "[" => *nesting += 1,
            "]" if *nesting > 0 => *nesting -= 1,
            _ => {}
        }
    }
}

impl InnerNode {
    /// Update this node after one or more of its children were replaced or
    /// updated. The new children must already be in `self.children`.
    ///
    /// `prev_*` variables should come from the previous range of children.
    /// `new_*` are for the new range of children.
    fn update_parent(
        &mut self,
        prev_len: usize,
        new_len: usize,
        prev_descendants: usize,
        new_descendants: usize,
        prev_diagnosis: Diagnosis,
        new_diagnosis: Diagnosis,
    ) {
        // Length and descendants are numerical, can update with algebra.
        // Subtract the previous values first to avoid potential overflow.
        self.len = (self.len - prev_len) + new_len;
        self.descendants = (self.descendants - prev_descendants) + new_descendants;

        // Our diagnosis is the `OR` of all our children's diagnoses.
        if !self.diagnosis.either() || new_diagnosis.both() {
            // We can avoid iterating our children if our previous overall state
            // had no errors/warnings, or if the new children have both.
            self.diagnosis = new_diagnosis;
        } else if prev_diagnosis != new_diagnosis {
            // Otherwise, we recompute using all the children since we don't
            // know if there were multiple children with errors or warnings.
            self.diagnosis = Diagnosis::any(&self.children);
        }
    }

    /// Replace a range of children and calls [`Self::update_parent`].
    /// [`Self::renumber_children`] should be called immediately after this.
    ///
    /// To increase the stability of span numbers, this does not replace nodes
    /// at the front/back that are equal to the previous nodes. Returns the
    /// final indices of the inserted nodes.
    pub fn replace_children(
        &mut self,
        mut range: Range<usize>,
        mut replacement: Vec<SyntaxNode>,
    ) -> Range<usize> {
        let mut replacement_range = 0..replacement.len();

        // Trim off common prefix.
        while range.start < range.end
            && replacement_range.start < replacement_range.end
            && self.children[range.start]
                .spanless_eq(&replacement[replacement_range.start])
        {
            range.start += 1;
            replacement_range.start += 1;
        }

        // Trim off common suffix.
        while range.start < range.end
            && replacement_range.start < replacement_range.end
            && self.children[range.end - 1]
                .spanless_eq(&replacement[replacement_range.end - 1])
        {
            range.end -= 1;
            replacement_range.end -= 1;
        }

        // Compute child information for the `update_parent` call.
        let new = &replacement[replacement_range.clone()];
        let new_len = new.iter().map(SyntaxNode::len).sum::<usize>();
        let new_desc = new.iter().map(SyntaxNode::descendants).sum::<usize>();
        let new_diag = Diagnosis::any(new);

        let prev = &self.children[range.clone()];
        let prev_len = prev.iter().map(SyntaxNode::len).sum::<usize>();
        let prev_desc = prev.iter().map(SyntaxNode::descendants).sum::<usize>();
        let prev_diag = Diagnosis::any(prev);

        let replaced_range = range.start..range.start + new.len();

        self.children.splice(range, replacement.drain(replacement_range));
        self.update_parent(prev_len, new_len, prev_desc, new_desc, prev_diag, new_diag);

        replaced_range
    }

    /// Try to assign span numbers to a range of children. If there are not
    /// enough numbers, retries taking exponentially more children into account.
    pub fn renumber_children(
        &mut self,
        span: Span,
        range: Range<usize>,
    ) -> NumberingResult {
        let Some(id) = span.id() else { return Err(Unnumberable) };

        let mut left = 0;
        let mut right = 0;
        let max_left = range.start;
        let max_right = self.children.len() - range.end;
        loop {
            let renumber = range.start - left..range.end + right;

            // The minimum assignable number is either
            // - the upper bound of the node right before the to-be-renumbered
            //   children,
            // - or this inner node's span number plus one if renumbering starts
            //   at the first child.
            let start_number = renumber
                .start
                .checked_sub(1)
                .and_then(|i| self.children.get(i))
                .map_or(span.number() + 1, |child| child.upper());

            // The upper bound for renumbering is either
            // - the span number of the first child after the to-be-renumbered
            //   children,
            // - or this node's upper bound if renumbering ends behind the last
            //   child.
            let end_number = self
                .children
                .get(renumber.end)
                .map_or(self.upper, |next| next.span().number());

            // Try to renumber.
            let within = start_number..end_number;
            if self.numberize(None, id, Some(renumber), within).is_ok() {
                return Ok(());
            }

            // If it didn't even work with all children, we give up.
            if left == max_left && right == max_right {
                return Err(Unnumberable);
            }

            // Exponential expansion to both sides.
            left = (left + 1).next_power_of_two().min(max_left);
            right = (right + 1).next_power_of_two().min(max_right);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;
    use crate::{FileId, PreferredCompilerVersion, Source};

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
        /// Insert at the index after the first match of this string.
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

    /// Assert that all spans in the tree are valid and not detached.
    #[track_caller]
    fn check_valid_spans(root: &SyntaxNode) {
        root.traverse(|node| {
            assert!(
                !node.span().is_detached(),
                "found a detached span for {:?}: '{}'",
                node.kind(),
                node.full_text(),
            );
            node.children()
        });
    }

    #[track_caller]
    fn test(text: &str, edit: Edit, with: &str, expected: Reparse) {
        // TODO(tinger): Test with string interpolation?
        let mut source = Source::new(
            FileId::new_for_test("test_reparse"),
            text,
            PreferredCompilerVersion::default(),
        );
        let orig_tree = source.root().clone();
        // `Source::edit()` is the public interface for reparsing.
        let replaced_range =
            source.edit(edit.into_range(text), with, PreferredCompilerVersion::default());
        let reparsed_tree = source.root().clone();
        check_valid_spans(&orig_tree);
        check_valid_spans(&reparsed_tree);
        let normal_parse = parse(source.text(), PreferredCompilerVersion::default());
        assert!(normal_parse.span().is_detached(), "should be detached");
        if !reparsed_tree.spanless_eq(&normal_parse) {
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
        test("#set thing\"a\n", Edit::End, "\"", Incr("\"a\n\""));
        test("#let x = \"a\nb\nc", Edit::End, "\"", All);
        test("#let x = {\"a\nb\nc}", Edit::After("c"), "\"", Incr("{\"a\nb\nc\"}"));
        test("#if x {\"a\nb\nc", Edit::End, "\"", All);
        test("#if x \"a\nb\nc", Edit::End, "\"", Incr("b\nc\""));
        test("#if x \"a\nb\nc", Edit::After("x "), "{}", Incr("#if x {}\"a\n"));
        test("#if x $a\nb\nc", Edit::End, "$", All);
        test("#for x in \"a\nb\nc", Edit::End, "\"", All);
        test("#for x \"a\nb\nc", Edit::After("x "), "in ", All);
    }

    /// Test basic reparsing correctness when modifying, inserting, or removing
    /// syntax warnings.
    #[test]
    fn test_reparse_syntax_warnings() {
        use Reparse::*;
        test("*strong*", Edit::Match("strong"), "", All);
        test("**", Edit::At(1), "strong", All);
        test("**", Edit::At(1), "__", All);
        test("```warn--empty```", Edit::Match("warn-"), "still", All);
        test("```yes-empty```", Edit::Match("yes-"), "not ", All);
    }

    /// Test reparsing with deeply nested code and markup blocks.
    ///
    /// FUTURE: Above `n == 127`, normal parsing hits the max parsing depth, but
    /// reparsing doesn't since we don't track the depth anywhere right now.
    /// This is a bug (it yields different trees when reparsing), but it will
    /// require a bit more effort to solve.
    #[test]
    fn test_reparse_nested_code() {
        let mut text = "#{A;B}".to_string();
        let mut prev_n = 1_usize;
        // Values chosen mostly arbitrarily, but some had interesting span
        // numbering differences when originally experimenting.
        for n in [10, 19, 20, 21, 22, 30, 50, 51, 62, 63, 100, 127] {
            for _ in prev_n..n {
                text.insert(1, '{');
                text.push('}');
            }
            prev_n = n;
            println!("n: {n}");

            // Decrease the number of nodes:
            test(&text, Edit::Match(";"), "1", Reparse::Incr("{A1B}"));
            test(&text, Edit::Match("A;B"), "2", Reparse::Incr("{2}"));

            // Leave the number of nodes the same:
            test(&text, Edit::Match("A;B"), "A;B", Reparse::Incr("{A;B}"));
            test(&text, Edit::Match("A;B"), "B;C", Reparse::Incr("{B;C}"));
            test(&text, Edit::Match(";"), "+", Reparse::Incr("{A+B}"));
            test(&text, Edit::After("A"), "3", Reparse::Incr("{A3;B}"));

            // Increase the number of nodes:
            test(&text, Edit::After("A"), ";5", Reparse::Incr("{A;5;B}"));
            test(&text, Edit::At(n + 3), "6;", Reparse::Incr("{A;6;B}"));
        }
    }

    /// Test reparsing with deeply nested code and markup blocks.
    #[test]
    fn test_reparse_nested_markup() {
        let mut text = "#[ A~B ]".to_string();
        let mut prev_n = 1_usize;
        // Values chosen mostly arbitrarily, but some had interesting span
        // numbering differences when originally experimenting.
        for n in [10, 19, 20, 21, 22, 30, 50, 51, 62, 63, 100, 127] {
            for _ in prev_n..n {
                text.insert_str(0, "#[");
                text.push(']');
            }
            prev_n = n;
            println!("n: {n}");

            // Decrease the number of nodes:
            test(&text, Edit::Match("~"), "1", Reparse::Incr(" A1B "));
            test(&text, Edit::Match("A~B"), "2", Reparse::Incr(" 2 "));

            // Leave the number of nodes the same:
            test(&text, Edit::Match("A~B"), "A~B", Reparse::Incr(" A~B "));
            test(&text, Edit::Match("A~B"), "3~B", Reparse::Incr(" 3~B "));
            test(&text, Edit::After("A"), "3", Reparse::Incr(" A3~B "));

            // Increase the number of nodes:
            test(&text, Edit::After("A"), "~4", Reparse::Incr(" A~4~B "));
            test(&text, Edit::At(2 * n + 3), "5~", Reparse::Incr(" A~5~B "));
        }
    }

    /// Don't incrementally reparse ranges inside a syntax warning.
    #[test]
    fn test_reparse_within_warnings() {
        // We build a custom tree with a warning since we don't currently
        // produce any syntax warnings that would trigger this bug.
        let text = "#[nor~mal] #[warn~ing]".to_string();
        let mut root = parse(&text, PreferredCompilerVersion::default());

        let root_inner = root.reparse_inner_node_mut().unwrap().0;
        let second_block = &mut root_inner.children[4];
        assert_eq!(second_block.full_text(), "[warn~ing]");
        // Note how the warning message can include text from within the node
        // arbitrarily. This is the primary reason reparsing within warnings is
        // unsound: there's no way to know how to update the message/hints!
        second_block.warn_at(5..6, "tilde between `warn` and `ing`: `~`");
        // We have to update the parent's diagnosis due to the new warning.
        let warning_diagnosis = Diagnosis { warnings: true, errors: false };
        // `{new,prev}_{len,desc}` are 0 since only their difference matters.
        root_inner.update_parent(0, 0, 0, 0, Diagnosis::default(), warning_diagnosis);
        assert!(root.diagnosis().warnings);

        // Assign initial span numbers so we can properly reparse.
        let id = FileId::new_for_test("reparse_within_warnings");
        root.numberize(id, Span::FULL).unwrap();
        let mut source = Source::with_root(id, text, root);

        // First we edit and reparse `nor~mal` into `normal`. The reparsed range
        // should stay within the content block.
        let replaced = Edit::Match("r~m").into_range(source.text());
        let replace_with = "rm";
        let reparsed =
            source.edit(replaced, replace_with, PreferredCompilerVersion::default());
        assert_eq!(&source.text()[reparsed], "normal");
        // Reparsing `normal` should not affect the warning:
        assert!(source.root().diagnosis().warnings);

        // Then we edit and reparse `warn~ing` into `warning`. Even though the
        // edit is fully inside the warned content block, the reparsed range
        // should expand to the outer code block since we can't soundly reparse
        // within warnings.
        let replaced = Edit::Match("n~i").into_range(source.text());
        let replace_with = "ni";
        let reparsed =
            source.edit(replaced, replace_with, PreferredCompilerVersion::default());
        assert_eq!(&source.text()[reparsed], "#[normal] #[warning]");
        assert!(!source.root().diagnosis().warnings);
    }
}
