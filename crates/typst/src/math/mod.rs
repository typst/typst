//! Mathematical formulas.

#[macro_use]
mod ctx;

pub mod accent;

mod align;
mod attach;
mod cancel;
#[path = "class.rs"]
mod class_;
mod equation;
mod frac;
mod fragment;
mod lr;
mod matrix;
mod op;
mod root;
mod row;
mod spacing;
mod stretch;
mod style;
mod underover;

pub use self::accent::{Accent, AccentElem};
pub use self::align::*;
pub use self::attach::*;
pub use self::cancel::*;
pub use self::class_::*;
pub use self::equation::*;
pub use self::frac::*;
pub use self::lr::*;
pub use self::matrix::*;
pub use self::op::*;
pub use self::root::*;
pub use self::stretch::*;
pub use self::style::*;
pub use self::underover::*;

use self::ctx::*;
use self::fragment::*;
use self::row::*;
use self::spacing::*;

use crate::diag::SourceResult;
use crate::foundations::{category, Category, Module, Scope, StyleChain};
use crate::text::TextElem;

/// Typst has special [syntax]($syntax/#math) and library functions to typeset
/// mathematical formulas. Math formulas can be displayed inline with text or as
/// separate blocks. They will be typeset into their own block if they start and
/// end with at least one space (e.g. `[$ x^2 $]`).
///
/// # Variables
/// In math, single letters are always displayed as is. Multiple letters,
/// however, are interpreted as variables and functions. To display multiple
/// letters verbatim, you can place them into quotes and to access single letter
/// variables, you can use the [hash syntax]($scripting/#expressions).
///
/// ```example
/// $ A = pi r^2 $
/// $ "area" = pi dot "radius"^2 $
/// $ cal(A) :=
///     { x in RR | x "is natural" } $
/// #let x = 5
/// $ #x < 17 $
/// ```
///
/// # Symbols
/// Math mode makes a wide selection of [symbols]($category/symbols/sym) like
/// `pi`, `dot`, or `RR` available. Many mathematical symbols are available in
/// different variants. You can select between different variants by applying
/// [modifiers]($symbol) to the symbol. Typst further recognizes a number of
/// shorthand sequences like `=>` that approximate a symbol. When such a
/// shorthand exists, the symbol's documentation lists it.
///
/// ```example
/// $ x < y => x gt.eq.not y $
/// ```
///
/// # Line Breaks
/// Formulas can also contain line breaks. Each line can contain one or multiple
/// _alignment points_ (`&`) which are then aligned.
///
/// ```example
/// $ sum_(k=0)^n k
///     &= 1 + ... + n \
///     &= (n(n+1)) / 2 $
/// ```
///
/// # Function calls
/// Math mode supports special function calls without the hash prefix. In these
/// "math calls", the argument list works a little differently than in code:
///
/// - Within them, Typst is still in "math mode". Thus, you can write math
///   directly into them, but need to use hash syntax to pass code expressions
///   (except for strings, which are available in the math syntax).
/// - They support positional and named arguments, but don't support trailing
///   content blocks and argument spreading.
/// - They provide additional syntax for 2-dimensional argument lists. The
///   semicolon (`;`) merges preceding arguments separated by commas into an
///   array argument.
///
/// ```example
/// $ frac(a^2, 2) $
/// $ vec(1, 2, delim: "[") $
/// $ mat(1, 2; 3, 4) $
/// $ lim_x =
///     op("lim", limits: #true)_x $
/// ```
///
/// To write a verbatim comma or semicolon in a math call, escape it with a
/// backslash. The colon on the other hand is only recognized in a special way
/// if directly preceded by an identifier, so to display it verbatim in those
/// cases, you can just insert a space before it.
///
/// Functions calls preceded by a hash are normal code function calls and not
/// affected by these rules.
///
/// # Alignment
/// When equations include multiple _alignment points_ (`&`), this creates
/// blocks of alternatingly right- and left-aligned columns. In the example
/// below, the expression `(3x + y) / 7` is right-aligned and `= 9` is
/// left-aligned. The word "given" is also left-aligned because `&&` creates two
/// alignment points in a row, alternating the alignment twice. `& &` and `&&`
/// behave exactly the same way. Meanwhile, "multiply by 7" is right-aligned
/// because just one `&` precedes it. Each alignment point simply alternates
/// between right-aligned/left-aligned.
///
/// ```example
/// $ (3x + y) / 7 &= 9 && "given" \
///   3x + y &= 63 & "multiply by 7" \
///   3x &= 63 - y && "subtract y" \
///   x &= 21 - y/3 & "divide by 3" $
/// ```
///
/// # Math fonts
/// You can set the math font by with a [show-set rule]($styling/#show-rules) as
/// demonstrated below. Note that only special OpenType math fonts are suitable
/// for typesetting maths.
///
/// ```example
/// #show math.equation: set text(font: "Fira Math")
/// $ sum_(i in NN) 1 + i $
/// ```
///
/// # Math module
/// All math functions are part of the `math` [module]($scripting/#modules),
/// which is available by default in equations. Outside of equations, they can
/// be accessed with the `math.` prefix.
#[category]
pub static MATH: Category;

/// Create a module with all math definitions.
pub fn module() -> Module {
    let mut math = Scope::deduplicating();
    math.category(MATH);
    math.define_elem::<EquationElem>();
    math.define_elem::<TextElem>();
    math.define_elem::<LrElem>();
    math.define_elem::<MidElem>();
    math.define_elem::<AttachElem>();
    math.define_elem::<StretchElem>();
    math.define_elem::<ScriptsElem>();
    math.define_elem::<LimitsElem>();
    math.define_elem::<AccentElem>();
    math.define_elem::<UnderlineElem>();
    math.define_elem::<OverlineElem>();
    math.define_elem::<UnderbraceElem>();
    math.define_elem::<OverbraceElem>();
    math.define_elem::<UnderbracketElem>();
    math.define_elem::<OverbracketElem>();
    math.define_elem::<UnderparenElem>();
    math.define_elem::<OverparenElem>();
    math.define_elem::<UndershellElem>();
    math.define_elem::<OvershellElem>();
    math.define_elem::<CancelElem>();
    math.define_elem::<FracElem>();
    math.define_elem::<BinomElem>();
    math.define_elem::<VecElem>();
    math.define_elem::<MatElem>();
    math.define_elem::<CasesElem>();
    math.define_elem::<RootElem>();
    math.define_elem::<ClassElem>();
    math.define_elem::<OpElem>();
    math.define_elem::<PrimesElem>();
    math.define_func::<abs>();
    math.define_func::<norm>();
    math.define_func::<round>();
    math.define_func::<sqrt>();
    math.define_func::<upright>();
    math.define_func::<bold>();
    math.define_func::<italic>();
    math.define_func::<serif>();
    math.define_func::<sans>();
    math.define_func::<cal>();
    math.define_func::<frak>();
    math.define_func::<mono>();
    math.define_func::<bb>();
    math.define_func::<display>();
    math.define_func::<inline>();
    math.define_func::<script>();
    math.define_func::<sscript>();

    // Text operators, spacings, and symbols.
    op::define(&mut math);
    spacing::define(&mut math);
    for (name, symbol) in crate::symbols::SYM {
        math.define(*name, symbol.clone());
    }

    Module::new("math", math)
}

/// Layout for math elements.
pub trait LayoutMath {
    /// Layout the element, producing fragment in the context.
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()>;
}
