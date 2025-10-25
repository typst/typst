use crate::foundations::{Content, NativeElement, SymbolElem, elem, func};
use crate::layout::{Length, Rel};
use crate::math::Mathy;

/// Scales delimiters.
///
/// While matched delimiters scale by default, this can be used to scale
/// unmatched delimiters and to control the delimiter scaling more precisely.
#[elem(title = "Left/Right", Mathy)]
pub struct LrElem {
    /// The size of the brackets, relative to the height of the wrapped content.
    #[default(Rel::one())]
    pub size: Rel<Length>,

    /// The delimited content, including the delimiters.
    #[required]
    #[parse(
        let mut arguments = args.all::<Content>()?.into_iter();
        let mut body = arguments.next().unwrap_or_default();
        arguments.for_each(|arg| body += SymbolElem::packed(',') + arg);
        body
    )]
    pub body: Content,
}

/// Scales delimiters vertically to the nearest surrounding `{lr()}` group.
///
/// ```example
/// $ { x mid(|) sum_(i=1)^n w_i|f_i (x)| < 1 } $
/// ```
#[elem(Mathy)]
pub struct MidElem {
    /// The content to be scaled.
    #[required]
    pub body: Content,
}

macro_rules! wrappers {
    {$(
        $( #[$meta:meta] )*
        $vis:vis $name:ident:
        $left:literal, $right:literal;
    )*} => {$(
        $( #[$meta] )*
        #[func]
        $vis fn $name(
            /// The size of the brackets, relative to the height of the wrapped
            /// content.
            ///
            /// Default: the current value of [`lr.size`]($math.lr.size).
            #[named]
            size: Option<Rel<Length>>,
            /// The expression to wrap.
            body: Content,
        ) -> Content {
            delimited(body, $left, $right, size)
        }
    )*}
}

wrappers! {
    /// Floors an expression.
    ///
    /// ```example
    /// $ floor(x/2) $
    /// ```
    pub floor: '⌊', '⌋';

    /// Ceils an expression.
    ///
    /// ```example
    /// $ ceil(x/2) $
    /// ```
    pub ceil: '⌈', '⌉';

    /// Rounds an expression.
    ///
    /// ```example
    /// $ round(x/2) $
    /// ```
    pub round: '⌊', '⌉';

    /// Takes the absolute value of an expression.
    ///
    /// ```example
    /// $ abs(x/2) $
    /// ```
    pub abs: '|', '|';

    /// Takes the norm of an expression.
    ///
    /// ```example
    /// $ norm(vec(1, 2)) $
    /// ```
    pub norm: '‖', '‖';

    // The following functions are not part of the public API. Instead, they are
    // accessible as symbols.

    pub paren: '(', ')';
    pub paren_flat: '⟮', '⟯';
    pub paren_closed: '⦇', '⦈';
    pub paren_stroked: '⦅', '⦆';

    pub brace: '{', '}';
    pub brace_stroked: '⦃', '⦄';

    pub bracket: '[', ']';
    pub bracket_top_tick: '⦍', '⦐';
    pub bracket_bottom_tick: '⦏', '⦎';
    pub bracket_stroked: '⟦', '⟧';

    pub shell: '❲', '❳';
    pub shell_stroked: '⟬', '⟭';
    pub shell_filled: '⦗', '⦘';

    pub bag: '⟅', '⟆';

    pub mustache: '⎰', '⎱';
    pub mustache_rev: '⎱', '⎰';

    pub fence: '⧘', '⧙';
    pub fence_double: '⧚', '⧛';

    pub chevron: '⟨', '⟩';
    pub chevron_curly: '⧼', '⧽';
    pub chevron_dot: '⦑', '⦒';
    pub chevron_closed: '⦉', '⦊';
    pub chevron_double: '⟪', '⟫';

    pub corner_top: '⌜', '⌝';
    pub corner_bottom: '⌞', '⌟';
}

fn delimited(
    body: Content,
    left: char,
    right: char,
    size: Option<Rel<Length>>,
) -> Content {
    let span = body.span();
    let mut elem = LrElem::new(Content::sequence([
        SymbolElem::packed(left),
        body,
        SymbolElem::packed(right),
    ]));
    // Push size only if size is provided
    if let Some(size) = size {
        elem.size.set(size);
    }
    elem.pack().spanned(span)
}
