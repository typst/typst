use std::ops::Range;
use std::rc::Rc;

use crate::syntax::{Green, GreenNode, NodeKind};

use super::{
    is_newline, parse, parse_atomic, parse_atomic_markup, parse_block, parse_comment,
    parse_markup, parse_markup_elements, parse_template, Scanner, TokenMode,
};

/// The conditions that a node has to fulfill in order to be replaced.
///
/// This can dictate if a node can be replaced at all and if yes, what can take
/// its place.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SuccessionRule {
    /// Changing this node can never have an influence on the other nodes.
    Safe,
    /// This node has to be replaced with a single token of the same kind.
    SameKind(Option<TokenMode>),
    /// In code mode, this node can only be changed into a single atomic
    /// expression, otherwise it is safe.
    AtomicPrimary,
    /// Changing an unsafe layer node in code mode changes what the parents or
    /// the surrounding nodes would be and is therefore disallowed. Change the
    /// parents or children instead. If it appears in Markup, however, it is
    /// safe to change.
    UnsafeLayer,
    /// Changing an unsafe node or any of its children is not allowed. Change
    /// the parents instead.
    Unsafe,
}

/// The conditions under which a node can be inserted or remain in a tree.
///
/// These conditions all search the neighbors of the node and see if its
/// existence is plausible with them present. This can be used to encode some
/// context-free language components for incremental parsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum NeighbourRule {
    /// These nodes depend on being at the start of a line. Reparsing of safe
    /// left neighbors has to check this invariant. Additionally, when
    /// exchanging the right sibling or inserting such a node the indentation of
    /// the first right non-trivia, non-whitespace sibling must not be greater
    /// than the current indentation.
    AtStart,
    /// These nodes depend on not being at the start of a line. Reparsing of
    /// safe left neighbors has to check this invariant. Otherwise, this node is
    /// safe.
    NotAtStart,
    /// These nodes could end up somewhere else up the tree if the parse was
    /// happening from scratch. The parse result has to be checked for such
    /// nodes. They are safe to add if followed up by other nodes.
    NotAtEnd,
    /// No additional requirements.
    None,
}

/// Allows partial refreshs of the [`Green`] node tree.
///
/// This struct holds a description of a change. Its methods can be used to try
/// and apply the change to a green tree.
pub struct Reparser<'a> {
    /// The new source code, with the change applied.
    src: &'a str,
    /// Which range in the old source file was changed.
    replace_range: Range<usize>,
    /// How many characters replaced the text in `replace_range`.
    replace_len: usize,
}

impl<'a> Reparser<'a> {
    /// Create a new reparser.
    pub fn new(src: &'a str, replace_range: Range<usize>, replace_len: usize) -> Self {
        Self { src, replace_range, replace_len }
    }
}

impl Reparser<'_> {
    /// Find the innermost child that is incremental safe.
    pub fn reparse(&self, green: &mut Rc<GreenNode>) -> Range<usize> {
        self.reparse_step(Rc::make_mut(green), 0, TokenMode::Markup, true)
            .unwrap_or_else(|| {
                *green = parse(self.src);
                0 .. self.src.len()
            })
    }

    fn reparse_step(
        &self,
        green: &mut GreenNode,
        mut offset: usize,
        parent_mode: TokenMode,
        mut outermost: bool,
    ) -> Option<Range<usize>> {
        let mode = green.kind().mode().unwrap_or(parent_mode);
        let child_mode = green.kind().mode().unwrap_or(TokenMode::Code);
        let original_count = green.children().len();

        // Save the current indent if this is a markup node.
        let indent = match green.kind() {
            NodeKind::Markup(n) => *n,
            _ => 0,
        };

        let mut first = None;
        let mut at_start = true;

        // Find the the first child in the range of children to reparse.
        for (i, child) in green.children_mut().iter_mut().enumerate() {
            let child_span = offset .. offset + child.len();

            // We look for the start in the element but we only take a position
            // at the right border if this is markup or the last element.
            //
            // This is because in Markup mode, we want to examine all nodes
            // touching a replacement but in code we want to atomically replace.
            if child_span.contains(&self.replace_range.start)
                || (mode == TokenMode::Markup
                    && self.replace_range.start == child_span.end)
            {
                first = Some((i, offset));
                break;
            }

            offset += child.len();
            at_start = child.kind().is_at_start(at_start);
        }

        let (first_idx, first_start) = first?;
        let mut last = None;

        // Find the the last child in the range of children to reparse.
        for (i, child) in green.children_mut().iter_mut().enumerate().skip(first_idx) {
            let child_span = offset .. offset + child.len();

            // Similarly to above, the end of the edit must be in the node but
            // if it is at the edge and we are in markup node, we also want its
            // neighbor!
            if child_span.contains(&self.replace_range.end)
                || self.replace_range.end == child_span.end
                    && (mode != TokenMode::Markup || i + 1 == original_count)
            {
                outermost &= i + 1 == original_count;
                last = Some((i, offset + child.len()));
                break;
            } else if mode != TokenMode::Markup
                || !child.kind().succession_rule().safe_in_markup()
            {
                break;
            }

            offset += child.len();
        }

        let (last_idx, last_end) = last?;
        let superseded_range = first_idx .. last_idx + 1;
        let superseded_span = first_start .. last_end;
        let last_kind = green.children()[last_idx].kind().clone();

        // First, we try if the child itself has another, more specific
        // applicable child.
        if superseded_range.len() == 1 {
            let child = &mut green.children_mut()[superseded_range.start];
            let prev_len = child.len();

            if last_kind.succession_rule() != SuccessionRule::Unsafe {
                if let Some(range) = match child {
                    Green::Node(node) => self.reparse_step(
                        Rc::make_mut(node),
                        first_start,
                        child_mode,
                        outermost,
                    ),
                    Green::Token(_) => None,
                } {
                    let new_len = child.len();
                    green.update_parent(new_len, prev_len);
                    return Some(range);
                }
            }
        }

        // We only replace multiple children in markup mode.
        if superseded_range.len() > 1 && mode == TokenMode::Code {
            return None;
        }

        // We now have a child that we can replace and a function to do so.
        let func = last_kind.reparsing_func(child_mode, indent)?;
        let succession = last_kind.succession_rule();

        let mut markup_min_column = 0;

        // If this is a markup node, we want to save its indent instead to pass
        // the right indent argument.
        if superseded_range.len() == 1 {
            let child = &mut green.children_mut()[superseded_range.start];
            if let NodeKind::Markup(n) = child.kind() {
                markup_min_column = *n;
            }
        }

        // The span of the to-be-reparsed children in the new source.
        let newborn_span = superseded_span.start
            ..
            superseded_span.end + self.replace_len - self.replace_range.len();

        // For atomic primaries we need to pass in the whole remaining string to
        // check whether the parser would eat more stuff illicitly.
        let reparse_span = if succession == SuccessionRule::AtomicPrimary {
            newborn_span.start .. self.src.len()
        } else {
            newborn_span.clone()
        };

        let mut prefix = "";
        for (i, c) in self.src[.. reparse_span.start].char_indices().rev() {
            if is_newline(c) {
                break;
            }
            prefix = &self.src[i .. reparse_span.start];
        }

        // Do the reparsing!
        let (mut newborns, terminated) = func(
            &prefix,
            &self.src[reparse_span.clone()],
            at_start,
            markup_min_column,
        )?;

        // Make sure that atomic primaries ate only what they were supposed to.
        if succession == SuccessionRule::AtomicPrimary {
            let len = newborn_span.len();
            if newborns.len() > 1 && newborns[0].len() == len {
                newborns.truncate(1);
            } else if newborns.iter().map(Green::len).sum::<usize>() != len {
                return None;
            }
        }

        // Do not accept unclosed nodes if the old node wasn't at the right edge
        // of the tree.
        if !outermost && !terminated {
            return None;
        }

        // If all post- and preconditions match, we are good to go!
        if validate(
            green.children(),
            superseded_range.clone(),
            at_start,
            &newborns,
            mode,
            succession,
            newborn_span.clone(),
            self.src,
        ) {
            green.replace_children(superseded_range, newborns);
            Some(newborn_span)
        } else {
            None
        }
    }
}

/// Validate that a node replacement is allowed by post- and preconditions.
fn validate(
    superseded: &[Green],
    superseded_range: Range<usize>,
    mut at_start: bool,
    newborns: &[Green],
    mode: TokenMode,
    post: SuccessionRule,
    newborn_span: Range<usize>,
    src: &str,
) -> bool {
    // Atomic primaries must only generate one new child.
    if post == SuccessionRule::AtomicPrimary && newborns.len() != 1 {
        return false;
    }

    // Same kind in mode `inside` must generate only one child and that child
    // must be of the same kind as previously.
    if let SuccessionRule::SameKind(inside) = post {
        let superseded_kind = superseded[superseded_range.start].kind();
        let superseded_mode = superseded_kind.mode().unwrap_or(mode);
        if inside.map_or(true, |m| m == superseded_mode)
            && (newborns.len() != 1 || superseded_kind != newborns[0].kind())
        {
            return false;
        }
    }

    // Neighbor invariants are only relevant in markup mode.
    if mode == TokenMode::Code {
        return true;
    }

    // Check if there are any `AtStart` predecessors which require a certain
    // indentation.
    let s = Scanner::new(src);
    let mut prev_pos = newborn_span.start;
    for child in (&superseded[.. superseded_range.start]).iter().rev() {
        prev_pos -= child.len();
        if !child.kind().is_trivia() {
            if child.kind().neighbour_rule() == NeighbourRule::AtStart {
                let left_col = s.column(prev_pos);

                // Search for the first non-trivia newborn.
                let mut new_pos = newborn_span.start;
                let mut child_col = None;
                for child in newborns {
                    if !child.kind().is_trivia() {
                        child_col = Some(s.column(new_pos));
                        break;
                    }

                    new_pos += child.len();
                }

                if let Some(child_col) = child_col {
                    if child_col > left_col {
                        return false;
                    }
                }
            }

            break;
        }
    }

    // Compute the at_start state behind the new children.
    for child in newborns {
        at_start = child.kind().is_at_start(at_start);
    }

    // Ensure that a possible at-start or not-at-start precondition of
    // a node after the replacement range is satisfied.
    for child in &superseded[superseded_range.end ..] {
        let neighbour_rule = child.kind().neighbour_rule();
        if (neighbour_rule == NeighbourRule::AtStart && !at_start)
            || (neighbour_rule == NeighbourRule::NotAtStart && at_start)
        {
            return false;
        }

        if !child.kind().is_trivia() {
            break;
        }

        at_start = child.kind().is_at_start(at_start);
    }

    // Verify that the last of the newborns is not `NotAtEnd`.
    if newborns.last().map_or(false, |child| {
        child.kind().neighbour_rule() == NeighbourRule::NotAtEnd
    }) {
        return false;
    }

    // We have to check whether the last non-trivia newborn is `AtStart` and
    // verify the indent of its right neighbors in order to make sure its
    // indentation requirements are fulfilled.
    let mut child_pos = newborn_span.end;
    for child in newborns.iter().rev() {
        child_pos -= child.len();

        if child.kind().is_trivia() {
            continue;
        }

        if child.kind().neighbour_rule() == NeighbourRule::AtStart {
            let child_col = s.column(child_pos);

            let mut right_pos = newborn_span.end;
            for child in &superseded[superseded_range.end ..] {
                if child.kind().is_trivia() {
                    right_pos += child.len();
                    continue;
                }

                if s.column(right_pos) > child_col {
                    return false;
                }
                break;
            }
        }
        break;
    }

    true
}

impl NodeKind {
    /// Return the correct reparsing function given the postconditions for the
    /// type.
    fn reparsing_func(
        &self,
        parent_mode: TokenMode,
        indent: usize,
    ) -> Option<fn(&str, &str, bool, usize) -> Option<(Vec<Green>, bool)>> {
        let mode = self.mode().unwrap_or(parent_mode);
        match self.succession_rule() {
            SuccessionRule::Unsafe | SuccessionRule::UnsafeLayer => None,
            SuccessionRule::AtomicPrimary if mode == TokenMode::Code => {
                Some(parse_atomic)
            }
            SuccessionRule::AtomicPrimary => Some(parse_atomic_markup),
            SuccessionRule::SameKind(x) if x == None || x == Some(mode) => match self {
                NodeKind::Markup(_) => Some(parse_markup),
                NodeKind::Template => Some(parse_template),
                NodeKind::Block => Some(parse_block),
                NodeKind::LineComment | NodeKind::BlockComment => Some(parse_comment),
                _ => None,
            },
            _ => match mode {
                TokenMode::Markup if indent == 0 => Some(parse_markup_elements),
                _ => return None,
            },
        }
    }

    /// Whether it is safe to do incremental parsing on this node.
    pub fn succession_rule(&self) -> SuccessionRule {
        match self {
            // These are all replaceable by other tokens.
            Self::Linebreak
            | Self::Text(_)
            | Self::TextInLine(_)
            | Self::NonBreakingSpace
            | Self::EnDash
            | Self::EmDash
            | Self::Escape(_)
            | Self::Strong
            | Self::Emph
            | Self::Heading
            | Self::Enum
            | Self::List
            | Self::Math(_) => SuccessionRule::Safe,

            // Only markup is expected at the points where it does occur. The
            // indentation must be preserved as well, also for the children.
            Self::Markup(_) => SuccessionRule::SameKind(None),

            // These can appear everywhere and must not change to other stuff
            // because that could change the outer expression.
            Self::LineComment | Self::BlockComment => SuccessionRule::SameKind(None),

            // These can appear as bodies and would trigger an error if they
            // became something else.
            Self::Template => SuccessionRule::SameKind(None),
            Self::Block => SuccessionRule::SameKind(Some(TokenMode::Code)),

            // Whitespace in code mode has to remain whitespace or else the type
            // of things would change.
            Self::Space(_) => SuccessionRule::SameKind(Some(TokenMode::Code)),

            // These are expressions that can be replaced by other expressions.
            Self::Ident(_)
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Length(_, _)
            | Self::Angle(_, _)
            | Self::Percentage(_)
            | Self::Str(_)
            | Self::Fraction(_)
            | Self::Array
            | Self::Dict
            | Self::Group
            | Self::None
            | Self::Auto => SuccessionRule::AtomicPrimary,

            // More complex, but still an expression.
            Self::ForExpr
            | Self::WhileExpr
            | Self::IfExpr
            | Self::LetExpr
            | Self::SetExpr
            | Self::ShowExpr
            | Self::WrapExpr
            | Self::ImportExpr
            | Self::IncludeExpr
            | Self::BreakExpr
            | Self::ContinueExpr
            | Self::ReturnExpr => SuccessionRule::AtomicPrimary,

            // These are complex expressions which may screw with their
            // environments.
            Self::Call
            | Self::Unary
            | Self::Binary
            | Self::CallArgs
            | Self::Named
            | Self::Spread => SuccessionRule::UnsafeLayer,

            // The closure is a bit magic with the let expression, and also it
            // is not atomic.
            Self::Closure | Self::ClosureParams => SuccessionRule::UnsafeLayer,

            // Missing these creates errors for the parents.
            Self::WithExpr | Self::ForPattern | Self::ImportItems => {
                SuccessionRule::UnsafeLayer
            }

            // Replacing parenthesis changes if the expression is balanced and
            // is therefore not safe.
            Self::LeftBracket
            | Self::RightBracket
            | Self::LeftBrace
            | Self::RightBrace
            | Self::LeftParen
            | Self::RightParen => SuccessionRule::Unsafe,

            // These work similar to parentheses.
            Self::Star | Self::Underscore => SuccessionRule::Unsafe,

            // Replacing an operator can change whether the parent is an
            // operation which makes it unsafe.
            Self::Comma
            | Self::Semicolon
            | Self::Colon
            | Self::Plus
            | Self::Minus
            | Self::Slash
            | Self::Eq
            | Self::EqEq
            | Self::ExclEq
            | Self::Lt
            | Self::LtEq
            | Self::Gt
            | Self::GtEq
            | Self::PlusEq
            | Self::HyphEq
            | Self::StarEq
            | Self::SlashEq
            | Self::Not
            | Self::And
            | Self::Or
            | Self::With
            | Self::Dots
            | Self::Arrow => SuccessionRule::Unsafe,

            // These keywords change what kind of expression the parent is and
            // how far the expression would go.
            Self::Let
            | Self::Set
            | Self::Show
            | Self::Wrap
            | Self::If
            | Self::Else
            | Self::For
            | Self::In
            | Self::As
            | Self::While
            | Self::Break
            | Self::Continue
            | Self::Return
            | Self::Import
            | Self::Include
            | Self::From => SuccessionRule::Unsafe,

            // This can affect whether strong or emph content ends.
            Self::Parbreak => SuccessionRule::Unsafe,

            // This element always has to remain in the same column so better
            // reparse the whole parent.
            Self::Raw(_) => SuccessionRule::Unsafe,

            // Changing the heading level, enum numbering, or list bullet
            // changes the next layer.
            Self::EnumNumbering(_) => SuccessionRule::Unsafe,

            // This can be anything, so we don't make any promises.
            Self::Error(_, _) | Self::Unknown(_) => SuccessionRule::Unsafe,
        }
    }

    /// Whether it is safe to insert this node next to some nodes or vice versa.
    pub fn neighbour_rule(&self) -> NeighbourRule {
        match self {
            Self::Heading | Self::Enum | Self::List => NeighbourRule::AtStart,
            Self::TextInLine(_) => NeighbourRule::NotAtStart,
            Self::Error(_, _) => NeighbourRule::NotAtEnd,
            _ => NeighbourRule::None,
        }
    }
}

impl SuccessionRule {
    /// Whether a node with this condition can be reparsed in markup mode.
    pub fn safe_in_markup(&self) -> bool {
        match self {
            Self::Safe | Self::UnsafeLayer => true,
            Self::SameKind(mode) => mode.map_or(false, |m| m != TokenMode::Markup),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::source::SourceFile;

    #[test]
    #[rustfmt::skip]
    fn test_incremental_parse() {
        #[track_caller]
        fn test(prev: &str, range: Range<usize>, with: &str, goal: Range<usize>) {
            let mut source = SourceFile::detached(prev);
            let range = source.edit(range, with);
            assert_eq!(range, goal);
            assert_eq!(parse(source.src()), *source.root());
        }

        // Test simple replacements.
        test("hello world", 6 .. 11, "walkers", 5 .. 13);
        test("some content", 0..12, "", 0..0);
        test("", 0..0, "do it", 0..5);
        test("a d e", 1 .. 3, " b c d", 0 .. 8);
        test("a #f() e", 1 .. 6, " b c d", 0 .. 8);
        test("{(0, 1, 2)}", 5 .. 6, "11pt", 5 .. 9);
        test("= A heading", 3 .. 3, "n evocative", 2 .. 22);
        test("your thing", 5 .. 5, "a", 4 .. 11);
        test("a your thing a", 6 .. 7, "a", 2 .. 12);
        test("{call(); abc}", 7 .. 7, "[]", 0 .. 15);
        test("#call() abc", 7 .. 7, "[]", 0 .. 10);
        test("hi[\n- item\n- item 2\n    - item 3]", 11 .. 11, "  ", 3 .. 34);
        test("hi\n- item\nno item\n    - item 3", 10 .. 10, "- ", 0 .. 32);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 16 .. 20, "none", 16 .. 20);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 33 .. 42, "[_gronk_]", 33 .. 42);
        test("#grid(columns: (auto, 1fr, 40%), [*plonk*], rect(width: 100%, height: 1pt, fill: conifer), [thing])", 34 .. 41, "_bar_", 34 .. 39);
        test("{let i=1; for x in range(5) {i}}", 6 .. 6, " ", 1 .. 9);
        test("{let i=1; for x in range(5) {i}}", 13 .. 14, "  ", 10 .. 32);
        test("hello {x}", 6 .. 9, "#f()", 5 .. 10);
        test("this is -- in my opinion -- spectacular", 8 .. 10, "---", 7 .. 12);
        test("understanding `code` is complicated", 15 .. 15, "C ", 0 .. 37);
        test("{ let x = g() }", 10 .. 12, "f(54", 2 .. 15);
        test("a #let rect with (fill: eastern)\nb", 16 .. 31, " (stroke: conifer", 2 .. 34);

        // Test the whitespace invariants.
        test("hello \\ world", 7 .. 8, "a ", 6 .. 14);
        test("hello \\ world", 7 .. 8, " a", 6 .. 14);
        test("x = y", 1 .. 1, " + y", 0 .. 6);
        test("x = y", 1 .. 1, " + y\n", 0 .. 10);
        test("abc\n= a heading\njoke", 3 .. 4, "\nmore\n\n", 0 .. 21);
        test("abc\n= a heading\njoke", 3 .. 4, "\nnot ", 0 .. 19);
        test("#let x = (1, 2 + ; Five\r\n\r", 19..22, "2.", 18..22);
        test("hey #myfriend", 4 .. 4, "\\", 0 .. 14);
        test("hey  #myfriend", 4 .. 4, "\\", 3 .. 6);

        // Test type invariants.
        test("a #for x in array {x}", 18 .. 21, "[#x]", 2 .. 22);
        test("a #let x = 1 {5}", 3 .. 6, "if", 0 .. 15);
        test("a {let x = 1 {5}} b", 3 .. 6, "if", 2 .. 16);
        test("#let x = 1 {5}", 4 .. 4, " if", 0 .. 17);
        test("{let x = 1 {5}}", 4 .. 4, " if", 0 .. 18);
        test("a // b c #f()", 3 .. 4, "", 0 .. 12);
        test("{\nf()\n//g(a)\n}", 6 .. 8, "", 0 .. 12);
        test("a{\nf()\n//g(a)\n}b", 7 .. 9, "", 1 .. 13);
        test("a #while x {\n g(x) \n}  b", 11 .. 11, "//", 0 .. 26);
        test("{(1, 2)}", 1 .. 1, "while ", 0 .. 14);
        test("a b c", 1 .. 1, "{[}", 0 .. 8);

        // Test unclosed things.
        test(r#"{"hi"}"#, 4 .. 5, "c", 0 .. 6);
        test(r"this \u{abcd}", 8 .. 9, "", 5 .. 12);
        test(r"this \u{abcd} that", 12 .. 13, "", 0 .. 17);
        test(r"{{let x = z}; a = 1} b", 6 .. 6, "//", 0 .. 24);
        test("a b c", 1 .. 1, " /* letters */", 0 .. 16);
        test("a b c", 1 .. 1, " /* letters", 0 .. 16);
        test("{if i==1 {a} else [b]; b()}", 12 .. 12, " /* letters */", 1 .. 35);
        test("{if i==1 {a} else [b]; b()}", 12 .. 12, " /* letters", 0 .. 38);

        // Test raw tokens.
        test(r#"a ```typst hello``` b"#, 16 .. 17, "", 0 .. 20);
        test(r#"a ```typst hello```"#, 16 .. 17, "", 0 .. 18);
    }
}
