use std::collections::HashMap;
use std::sync::LazyLock;

use bumpalo::Bump;
use comemo::Tracked;

use crate::engine::Engine;
use crate::foundations::{
    Args, CastInfo, Content, Context, Func, IntoValue, NativeElement, NativeFunc,
    NativeFuncData, NativeFuncPtr, ParamInfo, Reflect, Scope, SymbolElem, Type, elem,
    func,
};
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

/// Floors an expression.
///
/// ```example
/// $ floor(x/2) $
/// ```
#[func]
pub fn floor(
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Default: The current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to floor.
    body: Content,
) -> Content {
    delimited(body, '⌊', '⌋', size)
}

/// Ceils an expression.
///
/// ```example
/// $ ceil(x/2) $
/// ```
#[func]
pub fn ceil(
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Default: The current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to ceil.
    body: Content,
) -> Content {
    delimited(body, '⌈', '⌉', size)
}

/// Rounds an expression.
///
/// ```example
/// $ round(x/2) $
/// ```
#[func]
pub fn round(
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Default: The current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to round.
    body: Content,
) -> Content {
    delimited(body, '⌊', '⌉', size)
}

/// Takes the absolute value of an expression.
///
/// ```example
/// $ abs(x/2) $
/// ```
#[func]
pub fn abs(
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Default: The current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to take the absolute value of.
    body: Content,
) -> Content {
    delimited(body, '|', '|', size)
}

/// Takes the norm of an expression.
///
/// ```example
/// $ norm(x/2) $
/// ```
#[func]
pub fn norm(
    /// The size of the brackets, relative to the height of the wrapped content.
    ///
    /// Default: The current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to take the norm of.
    body: Content,
) -> Content {
    delimited(body, '‖', '‖', size)
}

/// Gets the Left/Right wrapper function corresponding to a left delimiter, if
/// any.
pub fn get_lr_wrapper_func(left: char) -> Option<Func> {
    match left {
        // Unlike `round`, `abs`, and `norm`, `floor` and `ceil` are of type
        // `symbol` and cast to a function like other L/R symbols. We could thus
        // rely on autogeneration for these as well, but since they are
        // specifically called out in the documentation on the L/R page (via the
        // group mechanism), it's nice for them to have a bit of extra
        // documentation.
        '⌈' => Some(ceil::func()),
        '⌊' => Some(floor::func()),
        l => FUNCS.get(&l).map(Func::from),
    }
}

/// The delimiter pairings supported for use as callable symbols.
const DELIMS: &[(char, char)] = &[
    // The `ceil` and `floor` pairs are omitted here because they are handled
    // manually.
    ('(', ')'),
    ('⟮', '⟯'),
    ('⦇', '⦈'),
    ('⦅', '⦆'),
    ('⦓', '⦔'),
    ('⦕', '⦖'),
    ('{', '}'),
    ('⦃', '⦄'),
    ('[', ']'),
    ('⦍', '⦐'),
    ('⦏', '⦎'),
    ('⟦', '⟧'),
    ('⦋', '⦌'),
    ('❲', '❳'),
    ('⟬', '⟭'),
    ('⦗', '⦘'),
    ('⟅', '⟆'),
    ('⎰', '⎱'),
    ('⎱', '⎰'),
    ('⧘', '⧙'),
    ('⧚', '⧛'),
    ('⟨', '⟩'),
    ('⧼', '⧽'),
    ('⦑', '⦒'),
    ('⦉', '⦊'),
    ('⟪', '⟫'),
    ('⌜', '⌝'),
    ('⌞', '⌟'),
];

/// Lazily created left/right wrapper functions.
static FUNCS: LazyLock<HashMap<char, NativeFuncData>> = LazyLock::new(|| {
    let bump = Box::leak(Box::new(Bump::new()));
    DELIMS
        .iter()
        .copied()
        .map(|(l, r)| (l, create_lr_func_data(l, r, bump)))
        .collect()
});

/// Creates metadata for an L/R wrapper function.
fn create_lr_func_data(left: char, right: char, bump: &'static Bump) -> NativeFuncData {
    let title = bumpalo::format!(in bump, "{}{} Left/Right", left, right).into_bump_str();
    let docs = bumpalo::format!(in bump, "Wraps an expression in {}{}.", left, right)
        .into_bump_str();
    NativeFuncData {
        function: NativeFuncPtr(bump.alloc(
            move |_: &mut Engine, _: Tracked<Context>, args: &mut Args| {
                let size = args.named("size")?;
                let body = args.expect("body")?;
                Ok(delimited(body, left, right, size).into_value())
            },
        )),
        name: "(..) => ..",
        title,
        docs,
        keywords: &[],
        contextual: false,
        scope: LazyLock::new(&|| Scope::new()),
        params: LazyLock::new(&|| create_lr_param_info()),
        returns: LazyLock::new(&|| CastInfo::Type(Type::of::<Content>())),
    }
}

/// Creates parameter signature metadata for an L/R function.
fn create_lr_param_info() -> Vec<ParamInfo> {
    vec![
        ParamInfo {
            name: "size",
            docs: "\
            The size of the brackets, relative to the height of the wrapped content.\n\
            \n\
            Default: The current value of [`lr.size`]($math.lr.size).",
            input: Rel::<Length>::input(),
            default: None,
            positional: false,
            named: true,
            variadic: false,
            required: false,
            settable: false,
        },
        ParamInfo {
            name: "body",
            docs: "The expression to wrap.",
            input: Content::input(),
            default: None,
            positional: true,
            named: false,
            variadic: false,
            required: true,
            settable: false,
        },
    ]
}

/// Creates an L/R element with the given delimiters.
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
