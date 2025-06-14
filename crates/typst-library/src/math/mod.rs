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

use crate::foundations::{elem, Content, Module, NativeElement, Scope};
use crate::layout::{Em, HElem};
use crate::text::TextElem;

// Spacings.
pub const THIN: Em = Em::new(1.0 / 6.0);
pub const MEDIUM: Em = Em::new(2.0 / 9.0);
pub const THICK: Em = Em::new(5.0 / 18.0);
pub const QUAD: Em = Em::new(1.0);
pub const WIDE: Em = Em::new(2.0);

/// Create a module with all math definitions.
pub fn module() -> Module {
    let mut math = Scope::deduplicating();
    math.start_category(crate::Category::Math);
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
    for (name, value) in &[
        ("thin", THIN),
        ("med", MEDIUM),
        ("thick", THICK),
        ("quad", QUAD),
        ("wide", WIDE),
    ] {
        let h = |em: Em| HElem::new(em.into()).pack();
        let mut scope = Scope::new();
        scope.define("neg", h(Em::new(-value.get())));
        let mut module = Module::new(*name, scope);
        module = module.with_content(h(*value));
        math.define(name, module);
    }

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
