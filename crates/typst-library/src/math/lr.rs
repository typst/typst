use crate::foundations::{Content, NativeElement, NativeFunc, SymbolElem, elem, func};
use crate::layout::{Em, Length, Ratio, Rel};
use crate::math::{Mathy, StretchSize};

pub const DELIM_SHORT_FALL: Em = Em::new(-0.1);

#[func(name = "x => x - 0.1em")]
pub const fn default_lr_size(base: Length) -> Rel {
    Rel {
        rel: Ratio::zero(),
        abs: Length { abs: base.abs, em: DELIM_SHORT_FALL },
    }
}

/// Scales delimiters.
///
/// While matched delimiters scale by default, this can be used to scale
/// unmatched delimiters and to control the delimiter scaling more precisely.
#[elem(title = "Left/Right", Mathy)]
pub struct LrElem {
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[default(<default_lr_size>::data().into())]
    pub size: StretchSize,

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
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[named]
    size: Option<StretchSize>,
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
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[named]
    size: Option<StretchSize>,
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
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[named]
    size: Option<StretchSize>,
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
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[named]
    size: Option<StretchSize>,
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
    /// The size of the delimiters, relative to the height of the wrapped
    /// content.
    ///
    /// See the [stretch documentation]($math.stretch.size) for more
    /// information on sizes.
    #[named]
    size: Option<StretchSize>,
    /// The expression to take the norm of.
    body: Content,
) -> Content {
    delimited(body, '‖', '‖', size)
}

fn delimited(
    body: Content,
    left: char,
    right: char,
    size: Option<StretchSize>,
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
