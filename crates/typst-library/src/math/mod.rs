//! Mathematical formulas.

pub mod accent;
mod attach;
mod cancel;
mod equation;
mod frac;
mod lr;
mod matrix;
mod op;
mod root;
mod style;
mod underover;

pub use self::accent::{Accent, AccentElem};
pub use self::attach::*;
pub use self::cancel::*;
pub use self::equation::*;
pub use self::frac::*;
pub use self::lr::*;
pub use self::matrix::*;
pub use self::op::*;
pub use self::root::*;
pub use self::style::*;
pub use self::underover::*;

use typst_utils::singleton;
use unicode_math_class::MathClass;

use crate::foundations::{
    category, elem, Category, Content, Module, NativeElement, Scope,
};
use crate::layout::{Em, HElem};
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

// Spacings.
pub const THIN: Em = Em::new(1.0 / 6.0);
pub const MEDIUM: Em = Em::new(2.0 / 9.0);
pub const THICK: Em = Em::new(5.0 / 18.0);
pub const QUAD: Em = Em::new(1.0);
pub const WIDE: Em = Em::new(2.0);

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

    // Text operators.
    op::define(&mut math);

    // Spacings.
    math.define("thin", HElem::new(THIN.into()).pack());
    math.define("med", HElem::new(MEDIUM.into()).pack());
    math.define("thick", HElem::new(THICK.into()).pack());
    math.define("quad", HElem::new(QUAD.into()).pack());
    math.define("wide", HElem::new(WIDE.into()).pack());

    // Symbols.
    crate::symbols::define_math(&mut math);

    Module::new("math", math)
}

/// Trait for recognizing math elements and auto-wrapping them in equations.
pub trait Mathy {}

/// A math alignment point: `&`, `&&`.
#[elem(title = "Alignment Point", Mathy)]
pub struct AlignPointElem {}

impl AlignPointElem {
    /// Get the globally shared alignment point element.
    pub fn shared() -> &'static Content {
        singleton!(Content, AlignPointElem::new().pack())
    }
}

/// Forced use of a certain math class.
///
/// This is useful to treat certain symbols as if they were of a different
/// class, e.g. to make a symbol behave like a relation. The class of a symbol
/// defines the way it is laid out, including spacing around it, and how its
/// scripts are attached by default. Note that the latter can always be
/// overridden using [`{limits}`](math.limits) and [`{scripts}`](math.scripts).
///
/// # Example
/// ```example
/// #let loves = math.class(
///   "relation",
///   sym.suit.heart,
/// )
///
/// $x loves y and y loves 5$
/// ```
#[elem(Mathy)]
pub struct ClassElem {
    /// The class to apply to the content.
    #[required]
    pub class: MathClass,

    /// The content to which the class is applied.
    #[required]
    pub body: Content,
}
