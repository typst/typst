use std::ops::Range;
use std::rc::Rc;

use super::{Green, GreenNode, NodeKind, Span};

use crate::parse::{
    parse_atomic, parse_atomic_markup, parse_block, parse_comment, parse_markup,
    parse_markup_elements, parse_template, TokenMode,
};

pub struct Reparser<'a> {
    src: &'a str,
    replace_range: Span,
    replace_len: usize,
}

impl<'a> Reparser<'a> {
    pub fn new(src: &'a str, replace_range: Span, replace_len: usize) -> Self {
        Self { src, replace_range, replace_len }
    }
}

impl Reparser<'_> {
    /// Find the innermost child that is incremental safe.
    pub fn incremental(&self, green: &mut GreenNode) -> Result<Range<usize>, ()> {
        self.incremental_int(green, 0, TokenMode::Markup, true)
    }

    fn incremental_int(
        &self,
        green: &mut GreenNode,
        mut offset: usize,
        parent_mode: TokenMode,
        outermost: bool,
    ) -> Result<Range<usize>, ()> {
        let kind = green.kind().clone();
        let mode = kind.mode().contextualize(parent_mode);

        let mut loop_result = None;
        let mut child_at_start = true;
        let last = green.children.len() - 1;
        let mut start = None;
        for (i, child) in green.children.iter_mut().enumerate() {
            let child_span =
                Span::new(self.replace_range.source, offset, offset + child.len());
            if child_span.surrounds(self.replace_range)
                && start.is_none()
                && ((self.replace_range.start != child_span.end
                    && self.replace_range.end != child_span.start)
                    || mode == TokenMode::Code
                    || i == last)
            {
                let old_len = child.len();
                // First, we try if the child has another, more specific applicable child.
                if !kind.post().unsafe_interior() {
                    if let Ok(range) = match child {
                        Green::Node(n) => self.incremental_int(
                            Rc::make_mut(n),
                            offset,
                            kind.mode().child_mode(),
                            i == last && outermost,
                        ),
                        Green::Token(_) => Err(()),
                    } {
                        let new_len = child.len();
                        green.update_child_len(new_len, old_len);
                        return Ok(range);
                    }
                }

                // This didn't work, so we try to self.replace_range the child at this
                // level.
                loop_result =
                    Some((i .. i + 1, child_span, i == last && outermost, child.kind()));
                break;
            } else if start.is_none()
                && child_span.contains(self.replace_range.start)
                && mode == TokenMode::Markup
                && child.kind().post().markup_safe()
            {
                start = Some((i, offset));
            } else if child_span.contains(self.replace_range.end)
                && (self.replace_range.end != child_span.end || i == last)
                && mode == TokenMode::Markup
                && child.kind().post().markup_safe()
            {
                if let Some((start, start_offset)) = start {
                    loop_result = Some((
                        start .. i + 1,
                        Span::new(
                            self.replace_range.source,
                            start_offset,
                            offset + child.len(),
                        ),
                        i == last && outermost,
                        child.kind(),
                    ));
                }
                break;
            } else if start.is_some()
                && (mode != TokenMode::Markup || !child.kind().post().markup_safe())
            {
                break;
            }

            offset += child.len();
            child_at_start = child.kind().is_at_start(child_at_start);
        }


        // We now have a child that we can self.replace_range and a function to do so if
        // the loop found any results at all.
        let (child_idx_range, child_span, child_outermost, func, policy) =
            loop_result.ok_or(()).and_then(|(a, b, c, child_kind)| {
                let (func, policy) =
                    child_kind.reparsing_function(kind.mode().child_mode());
                Ok((a, b, c, func?, policy))
            })?;

        let src_span = child_span.inserted(self.replace_range, self.replace_len);
        let recompile_range = if policy == Postcondition::AtomicPrimary {
            src_span.start .. self.src.len()
        } else {
            src_span.to_range()
        };

        let (mut new_children, unterminated) =
            func(&self.src[recompile_range], child_at_start).ok_or(())?;

        // Do not accept unclosed nodes if the old node did not use to be at the
        // right edge of the tree.
        if !child_outermost && unterminated {
            return Err(());
        }

        let insertion = match check_invariants(
            &new_children,
            green.children(),
            child_idx_range.clone(),
            child_at_start,
            mode,
            src_span,
            policy,
        ) {
            InvariantResult::Ok => Ok(new_children),
            InvariantResult::UseFirst => Ok(vec![std::mem::take(&mut new_children[0])]),
            InvariantResult::Error => Err(()),
        }?;

        green.replace_child_range(child_idx_range, insertion);

        Ok(src_span.to_range())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum InvariantResult {
    Ok,
    UseFirst,
    Error,
}

fn check_invariants(
    use_children: &[Green],
    old_children: &[Green],
    child_idx_range: Range<usize>,
    child_at_start: bool,
    mode: TokenMode,
    src_span: Span,
    policy: Postcondition,
) -> InvariantResult {
    let (new_children, ok) = if policy == Postcondition::AtomicPrimary {
        if use_children.iter().map(Green::len).sum::<usize>() == src_span.len() {
            (use_children, InvariantResult::Ok)
        } else if use_children.len() == 1 && use_children[0].len() == src_span.len() {
            (&use_children[0 .. 1], InvariantResult::UseFirst)
        } else {
            return InvariantResult::Error;
        }
    } else {
        (use_children, InvariantResult::Ok)
    };

    let child_mode = old_children[child_idx_range.start].kind().mode().child_mode();

    // Check if the children / child has the right type.
    let same_kind = match policy {
        Postcondition::SameKind(x) => x.map_or(true, |x| x == child_mode),
        _ => false,
    };

    if same_kind || policy == Postcondition::AtomicPrimary {
        if new_children.len() != 1 {
            return InvariantResult::Error;
        }

        if same_kind {
            if old_children[child_idx_range.start].kind() != new_children[0].kind() {
                return InvariantResult::Error;
            }
        }
    }

    // Check if the neighbor invariants are still true.
    if mode == TokenMode::Markup {
        if child_idx_range.start > 0 {
            if old_children[child_idx_range.start - 1].kind().pre()
                == Precondition::RightWhitespace
                && !new_children[0].kind().is_whitespace()
            {
                return InvariantResult::Error;
            }
        }

        if new_children.last().map(|x| x.kind().pre())
            == Some(Precondition::RightWhitespace)
            && old_children.len() > child_idx_range.end
        {
            if !old_children[child_idx_range.end].kind().is_whitespace() {
                return InvariantResult::Error;
            }
        }

        let mut new_at_start = child_at_start;
        for child in new_children {
            new_at_start = child.kind().is_at_start(new_at_start);
        }

        for child in &old_children[child_idx_range.end ..] {
            if child.kind().is_trivia() {
                new_at_start = child.kind().is_at_start(new_at_start);
                continue;
            }

            match child.kind().pre() {
                Precondition::AtStart if !new_at_start => {
                    return InvariantResult::Error;
                }
                Precondition::NotAtStart if new_at_start => {
                    return InvariantResult::Error;
                }
                _ => {}
            }
            break;
        }
    }

    ok
}

impl NodeKind {
    pub fn reparsing_function(
        &self,
        parent_mode: TokenMode,
    ) -> (
        Result<fn(&str, bool) -> Option<(Vec<Green>, bool)>, ()>,
        Postcondition,
    ) {
        let policy = self.post();
        let mode = self.mode().contextualize(parent_mode);

        match policy {
            Postcondition::Unsafe | Postcondition::UnsafeLayer => (Err(()), policy),
            Postcondition::AtomicPrimary if mode == TokenMode::Code => {
                (Ok(parse_atomic), policy)
            }
            Postcondition::AtomicPrimary => (Ok(parse_atomic_markup), policy),
            Postcondition::SameKind(x) if x == None || x == Some(mode) => {
                let parser: fn(&str, bool) -> _ = match self {
                    NodeKind::Template => parse_template,
                    NodeKind::Block => parse_block,
                    NodeKind::LineComment | NodeKind::BlockComment => parse_comment,
                    _ => return (Err(()), policy),
                };

                (Ok(parser), policy)
            }
            _ => {
                let parser: fn(&str, bool) -> _ = match mode {
                    TokenMode::Markup if self == &Self::Markup => parse_markup,
                    TokenMode::Markup => parse_markup_elements,
                    _ => return (Err(()), policy),
                };

                (Ok(parser), policy)
            }
        }
    }

    /// Whether it is safe to do incremental parsing on this node. Never allow
    /// non-termination errors if this is not already the last leaf node.
    pub fn post(&self) -> Postcondition {
        match self {
            // Replacing parenthesis changes if the expression is balanced and
            // is therefore not safe.
            Self::LeftBracket
            | Self::RightBracket
            | Self::LeftBrace
            | Self::RightBrace
            | Self::LeftParen
            | Self::RightParen => Postcondition::Unsafe,

            // Replacing an operator can change whether the parent is an
            // operation which makes it unsafe. The star can appear in markup.
            Self::Star
            | Self::Comma
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
            | Self::Arrow => Postcondition::Unsafe,

            // These keywords are literals and can be safely be substituted with
            // other expressions.
            Self::None | Self::Auto => Postcondition::AtomicPrimary,

            // These keywords change what kind of expression the parent is and
            // how far the expression would go.
            Self::Let
            | Self::Set
            | Self::If
            | Self::Else
            | Self::For
            | Self::In
            | Self::While
            | Self::Break
            | Self::Continue
            | Self::Return
            | Self::Import
            | Self::Include
            | Self::From => Postcondition::Unsafe,

            Self::Markup => Postcondition::SameKind(None),

            Self::Space(_) => Postcondition::SameKind(Some(TokenMode::Code)),

            // These are all replaceable by other tokens.
            Self::Parbreak
            | Self::Linebreak
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
            | Self::Raw(_)
            | Self::Math(_) => Postcondition::Safe,

            // Changing the heading level, enum numbering, or list bullet
            // changes the next layer.
            Self::EnumNumbering(_) => Postcondition::Unsafe,

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
            | Self::Group => Postcondition::AtomicPrimary,

            Self::Call
            | Self::Unary
            | Self::Binary
            | Self::CallArgs
            | Self::Named
            | Self::Spread => Postcondition::UnsafeLayer,

            // The closure is a bit magic with the let expression, and also it
            // is not atomic.
            Self::Closure | Self::ClosureParams => Postcondition::UnsafeLayer,

            // These can appear as bodies and would trigger an error if they
            // became something else.
            Self::Template => Postcondition::SameKind(None),
            Self::Block => Postcondition::SameKind(Some(TokenMode::Code)),

            Self::ForExpr
            | Self::WhileExpr
            | Self::IfExpr
            | Self::LetExpr
            | Self::SetExpr
            | Self::ImportExpr
            | Self::IncludeExpr => Postcondition::AtomicPrimary,

            Self::WithExpr | Self::ForPattern | Self::ImportItems => {
                Postcondition::UnsafeLayer
            }

            // These can appear everywhere and must not change to other stuff
            // because that could change the outer expression.
            Self::LineComment | Self::BlockComment => Postcondition::SameKind(None),

            Self::Error(_, _) | Self::Unknown(_) => Postcondition::Unsafe,
        }
    }

    /// The appropriate precondition for the type.
    pub fn pre(&self) -> Precondition {
        match self {
            Self::Heading | Self::Enum | Self::List => Precondition::AtStart,
            Self::TextInLine(_) => Precondition::NotAtStart,
            Self::Linebreak => Precondition::RightWhitespace,
            _ => Precondition::None,
        }
    }
}

/// This enum describes what conditions a node has for being replaced by a new
/// parse result.
///
/// Safe nodes are replaced by the new parse result from the respective mode.
/// They can be replaced by multiple tokens. If a token is inserted in Markup
/// mode and the next token would not be `at_start` there needs to be a forward
/// check for a `EnsureAtStart` node. If this fails, the parent has to be
/// reparsed. if the direct whitespace sibling of a `EnsureRightWhitespace` is
/// `Unsafe`. Similarly, if a `EnsureRightWhitespace` token is one of the last
/// tokens to be inserted, the edit is invalidated if there is no following
/// whitespace. The atomic nodes may only be replaced by other atomic nodes. The
/// unsafe layers cannot be used but allow children access, the unsafe nodes do
/// neither.
///
/// *Procedure:*
/// 1. Check if the node is safe - if unsafe layer recurse, if unsafe, return
///    None.
/// 2. Reparse with appropriate node kind and `at_start`.
/// 3. Check whether the topmost group is terminated and the range was
///    completely consumed, otherwise return None.
/// 4. Check if the type criteria are met.
/// 5. If the node is not at the end of the tree, check if Strings etc. are
///    terminated.
/// 6. If this is markup, check the following things:
///   - The `at_start` conditions of the next non-comment and non-space(0) node
///     are met.
///   - The first node is whitespace or the previous siblings are not
///     `EnsureRightWhitespace`.
///   - If any of those fails, return None.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Postcondition {
    /// Changing this node can never have an influence on the other nodes.
    Safe,
    /// This node has to be replaced with a single token of the same kind.
    SameKind(Option<TokenMode>),
    /// Changing this node into a single atomic expression is allowed if it
    /// appears in code mode, otherwise it is safe.
    AtomicPrimary,
    /// Changing an unsafe layer node changes what the parents or the
    /// surrounding nodes would be and is therefore disallowed. Change the
    /// parents or children instead. If it appears in Markup, however, it is
    /// safe to change.
    UnsafeLayer,
    /// Changing an unsafe node or any of its children will trigger undefined
    /// behavior. Change the parents instead.
    Unsafe,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Precondition {
    /// These nodes depend on being at the start of a line. Reparsing of safe
    /// left neighbors has to check this invariant. Otherwise, this node is
    /// safe.
    AtStart,
    /// These nodes depend on not being at the start of a line. Reparsing of
    /// safe left neighbors has to check this invariant. Otherwise, this node is
    /// safe.
    NotAtStart,
    /// These nodes must be followed by whitespace.
    RightWhitespace,
    /// No additional requirements.
    None,
}

impl Postcondition {
    pub fn unsafe_interior(&self) -> bool {
        match self {
            Self::Unsafe => true,
            _ => false,
        }
    }

    pub fn markup_safe(&self) -> bool {
        match self {
            Self::Safe | Self::UnsafeLayer => true,
            Self::SameKind(tm) => tm.map_or(false, |tm| tm != TokenMode::Markup),
            _ => false,
        }
    }
}
