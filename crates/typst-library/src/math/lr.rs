use crate::engine::Engine;
use crate::foundations::{
    Args, CastInfo, Content, Context, Func, NativeElement, NativeFunc, NativeFuncData,
    NativeFuncPtr, ParamInfo, Reflect, Scope, SymbolElem, Type, elem,
};
use crate::layout::{Length, Rel};
use crate::math::Mathy;
use bumpalo::Bump;
use comemo::Tracked;
use ecow::EcoString;
use std::collections::HashMap;
use std::sync::LazyLock;
use typst_macros::func;

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
    /// Default: the current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to floor.
    body: Content,
) -> Content {
    delimited(body, '⌊'.into(), '⌋'.into(), size)
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
    /// Default: the current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to ceil.
    body: Content,
) -> Content {
    delimited(body, '⌈'.into(), '⌉'.into(), size)
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
    /// Default: the current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to round.
    body: Content,
) -> Content {
    delimited(body, '⌊'.into(), '⌉'.into(), size)
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
    /// Default: the current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to take the absolute value of.
    body: Content,
) -> Content {
    delimited(body, '|'.into(), '|'.into(), size)
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
    /// Default: the current value of [`lr.size`]($math.lr.size).
    #[named]
    size: Option<Rel<Length>>,
    /// The expression to take the norm of.
    body: Content,
) -> Content {
    delimited(body, '‖'.into(), '‖'.into(), size)
}

/// Gets the Left/Right wrapper function corresponding to a left parenthesis, if
/// any.
pub fn get_lr_wrapper_func(left: &str) -> Option<Func> {
    match left {
        "⌈" => Some(ceil::func()),
        "⌊" => Some(floor::func()),
        l => FUNCS.get(l).map(Func::from),
    }
}

/// Lazily created left/right wrapper functions.
static FUNCS: LazyLock<HashMap<&'static str, NativeFuncData>> = LazyLock::new(|| {
    let bump = Box::leak(Box::new(Bump::new()));
    [
        ("(", ")"),
        ("⟮", "⟯"),
        ("⦇", "⦈"),
        ("⦅", "⦆"),
        ("{", "}"),
        ("⦃", "⦄"),
        ("[", "]"),
        ("⦍", "⦐"),
        ("⦏", "⦎"),
        ("⟦", "⟧"),
        ("❲", "❳"),
        ("⟬", "⟭"),
        ("⦗", "⦘"),
        ("⟅", "⟆"),
        ("⎰", "⎱"),
        ("⎱", "⎰"),
        ("⧘", "⧙"),
        ("⧚", "⧛"),
        ("⟨", "⟩"),
        ("⧼", "⧽"),
        ("⦑", "⦒"),
        ("⦉", "⦊"),
        ("⟪", "⟫"),
        ("⌜", "⌝"),
        ("⌞", "⌟"),
    ]
    .into_iter()
    .map(|(l, r)| (l, create_lr_func(l.into(), r.into(), bump)))
    .collect()
});

fn create_lr_func(
    left: EcoString,
    right: EcoString,
    bump: &'static Bump,
) -> NativeFuncData {
    NativeFuncData {
        function: NativeFuncPtr(bump.alloc(
            move |_: &mut Engine, _: Tracked<Context>, args: &mut Args| {
                let size = args.named("size")?;
                let body = args.expect("body")?;
                args.take().finish()?;
                let output = delimited(body, left.clone(), right.clone(), size);
                ::typst_library::foundations::IntoResult::into_result(output, args.span)
            },
        )),
        name: "(..) => ..",
        title: "",
        docs: "",
        keywords: &[],
        contextual: false,
        scope: LazyLock::new(&|| Scope::new()),
        params: LazyLock::new(&|| {
            vec![
                ParamInfo {
                    name: "size",
                    docs: "\
                    The size of the brackets, relative to the height of the wrapped content.\n\
                    \n\
                    Default: the current value of [`lr.size`]($math.lr.size).",
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
        }),
        returns: LazyLock::new(&|| CastInfo::Type(Type::of::<Content>())),
    }
}

fn delimited(
    body: Content,
    left: EcoString,
    right: EcoString,
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
