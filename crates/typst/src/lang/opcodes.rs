use std::num::NonZeroU32;

use comemo::Tracked;

use typst_syntax::Span;
use typst_utils::PicoStr;

use crate::engine::Engine;
use crate::diag::SourceResult;
use crate::foundations::{Context, Value};

pub use super::operands::{
    ModuleId, AccessId, ClosureId, LabelId, PatternId, Pointer, Readable, SpanId, Writable,
};

macro_rules! opcode_filter {
    ($(#[$sattr:meta])*
    $name:ident: enter $(-> $out:ty)? $(=> {
        $(
            $(#[$attr:meta])*
            $arg:ident: $arg_ty:ty
        ),* $(,)?
    })?) => {
        opcode_filter! {
            $(#[$sattr])*
            $name: enter_isr $(-> $out)? $(=> {
                $(
                    $(#[$attr])*
                    $arg: $arg_ty
                ),*,
            })?
        }
    };
    (
        $(#[$sattr:meta])*
    $name:ident: $snek:ident $(-> $out:ty)? $(=> {
        $(
            $(#[$attr:meta])*
            $arg:ident: $arg_ty:ty
        ),* $(,)?
    })?) => {
        pub fn $snek(&mut self, span: Span, $($($arg: impl Into<$arg_ty>,)*)? $(out: impl Into<$out>)?) {
            let opcode = $name {
                $($(
                    $arg: $arg.into(),
                )*)?
                $(
                    out: <_ as Into<$out>>::into(out),
                )?
            };

            self.insr(span, Opcode::$name(opcode));
        }
    };
}

macro_rules! opcode_struct {
    (
        $(#[$sattr:meta])*
        $name:ident $(-> $out:ty)? $(=> {
            $(
                $(#[$attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?
        })?
    ) => {
        $(#[$sattr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(packed)]
        pub struct $name {
            $(
                $(
                    $(#[$attr])*
                    pub $arg: $arg_ty,
                )*
            )?
            $(
                #[doc = "The output of the instruction."]
                pub out: $out,
            )?
        }
    };
}

macro_rules! opcodes {
    (
        $(
            $(#[$sattr:meta])*
            $name:ident: $snek:ident $(-> $out:ty)? $(=> {
                $(
                    $(#[$attr:meta])*
                    $arg:ident: $arg_ty:ty
                ),* $(,)?
            })?
        ),* $(,)?
    ) => {
        $(
            opcode_struct! {
                $(#[$sattr])*
                $name $(-> $out)? $(=> {
                    $(
                        $(#[$attr])*
                        $arg: $arg_ty
                    ),*
                })?
            }
        )*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Opcode {
            #[doc = "Indicates a flow event."]
            Flow = 0,
            $(
                $(#[$sattr])*
                $name($name)
            ),*
        }

        impl<'lib> crate::lang::compiler::Compiler<'lib> {
            $(
                opcode_filter! {
                    $(#[$sattr])*
                    $name: $snek $(-> $out)? $(=> {
                        $(
                            $(#[$attr])*
                            $arg: $arg_ty
                        ),*,
                    })?
                }
            )*
        }

        impl crate::lang::interpreter::Run for Opcode {
            fn run(
                &self,
                instructions: &[Opcode],
                spans: &[Span],
                span: Span,
                vm: &mut crate::lang::interpreter::Vm,
                engine: &mut Engine,
                context: Tracked<Context>,
                iterator: Option<&mut dyn Iterator<Item = Value>>
            ) -> SourceResult<()> {
                vm.next();

                let isr = vm.instruction_pointer();
                match self {
                    Self::Flow => Ok(()),
                    $(Self::$name($snek) => {
                        $snek.run(
                            &instructions[isr..],
                            &spans[isr..],
                            span,
                            vm,
                            engine,
                            context,
                            iterator
                        )
                    })*
                }
            }

        }
    }
}

opcodes! {
    // -----------------------------------------------------------------------------
    // --------------------------------- OPERATORS ---------------------------------
    // -----------------------------------------------------------------------------

    /// Adds two values together.
    Add: add -> Writable => {
        /// The left-hand side of the addition.
        lhs: Readable,
        /// The right-hand side of the addition.
        rhs: Readable,
    },

    /// Subtracts two values.
    Sub: sub -> Writable => {
        /// The left-hand side of the subtraction.
        lhs: Readable,
        /// The right-hand side of the subtraction.
        rhs: Readable,
    },

    /// Multiplies two values.
    Mul: mul -> Writable => {
        /// The left-hand side of the multiplication.
        lhs: Readable,
        /// The right-hand side of the multiplication.
        rhs: Readable,
    },

    /// Divides two values.
    Div: div -> Writable => {
        /// The left-hand side of the division.
        lhs: Readable,
        /// The right-hand side of the division.
        rhs: Readable,
    },

    /// Negates a value.
    Neg: neg -> Writable => {
        /// The value to negate.
        value: Readable,
    },

    /// Positivizes a value.
    Pos: pos -> Writable => {
        /// The value to negate.
        value: Readable,
    },

    /// Logical not.
    Not: not -> Writable => {
        /// The value to negate.
        value: Readable,
    },

    /// Greater than.
    Gt: gt -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Greater than or equal to.
    Geq: geq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Less than.
    Lt: lt -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Less than or equal to.
    Leq: leq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Equal to.
    Eq: eq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Not equal to.
    Neq: neq -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Whether the left-hand side is in the right-hand side.
    In: in_ -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Whether the left-hand side is not in the right-hand side.
    NotIn: not_in -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Logical and.
    And: and -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Logical or.
    Or: or -> Writable => {
        /// The left-hand side of the comparison.
        lhs: Readable,
        /// The right-hand side of the comparison.
        rhs: Readable,
    },

    /// Copies a value.
    CopyIsr: copy -> Writable => {
        /// The value to copy.
        value: Readable,
    },

    /// Creates a new [`Value::None`].
    None: none -> Writable => { },

    /// Creates a new [`Value::Auto`].
    Auto: auto -> Writable => { },

    // -----------------------------------------------------------------------------
    // ---------------------------------- ASSIGN -----------------------------------
    // -----------------------------------------------------------------------------

    /// Assign to a value.
    Assign: assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    },

    /// Assign and add to a value.
    AddAssign: add_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    },

    /// Assign and subtract from a value.
    SubAssign: sub_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    },

    /// Assign and multiply a value.
    MulAssign: mul_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    },

    /// Assign and divide a value.
    DivAssign: div_assign -> AccessId => {
        /// The value to assign.
        value: Readable,
    },

    /// Destructures a value into a pattern.
    Destructure: destructure -> PatternId => {
        /// The value to destructure.
        value: Readable,
    },

    // -----------------------------------------------------------------------------
    // ---------------------------------- STYLING ----------------------------------
    // -----------------------------------------------------------------------------

    /// Creates and applies a new set rule.
    Set: set => {
        /// The target to set the rule on.
        target: Readable,
        /// The arguments to supply to the set rule.
        args: Readable,
    },

    /// Creates and applies a new show rule.
    Show: show => {
        /// The selector for the value to show.
        selector: Option<Readable>,
        /// The transform to apply.
        transform: Readable,
    },

    ShowSet: show_set => {
        /// The selector for the value to show.
        selector: Option<Readable>,
        /// The target to set the rule on.
        target: Readable,
        /// The arguments to supply to the set rule.
        args: Readable,
    },

    // -----------------------------------------------------------------------------
    // ----------------------------- FUNCTIONS & FLOW ------------------------------
    // -----------------------------------------------------------------------------

    /// Create a new `context` scope.
    Contextual: contextual -> Writable => {
        /// The instantiated closure to run.
        closure: Readable,
    },

    /// Instantiates a module.
    ///
    /// This involves:
    /// - Getting the final path of the module.
    /// - Instantiating the module and loading all of the variables
    /// defined in the module.
    InstantiateModule: instantiate_module -> Writable => {
        /// The path to the module.
        path: Readable,
        /// The module to instantiate.
        module: ModuleId,
    },

    /// Import a value from a module.
    Import: import -> Writable => {
        /// The module to import from.
        module: Readable,
        /// The value to import.
        access: PicoStr,
    },

    /// Include a file's content.
    Include: include -> Writable => {
        /// The path to the file to include.
        path: Readable,
    },

    /// Instantiates a closure.
    ///
    /// This involves:
    /// - Capturing all necessary values.
    /// - Capturing the default values of named arguments.
    ///
    /// And produces a [`Func`] which can then be called.
    Instantiate: instantiate -> Writable => {
        /// The closure to instantiate.
        closure: ClosureId,
    },

    /// Calls a function.
    Call: call -> Writable => {
        /// The closure to call.
        closure: AccessId,
        /// The arguments to call the closure with.
        args: Readable,
        /// Whether the call is done in a math context.
        math: bool,
        /// Whether the call contains a trailing comma.
        trailing_comma: bool,
    },

    /// Accesses a field.
    Field: field -> Writable => {
        /// The value to access.
        access: AccessId,
    },

    /// Enters a new while loop scope with optional joining.
    While: while_ -> Writable => {
        /// The length of the scope to enter.
        len: u32,
        /// Whether the scope produces content.
        content: bool,
    },

    /// Enters a new iterator scope with optional joining.
    Iter: iter -> Writable => {
        /// The length of the scope to enter.
        len: u32,
        /// The value to iterate over.
        iterable: Readable,
        /// Whether the scope produces content.
        content: bool,
    },

    /// Queries the next value of an iterator.
    /// Returns from the iterator scope if the iterator is exhausted.
    Next: next -> Writable => { },

    /// Continues a loop.
    Continue: continue_ => {},

    /// Breaks out of a loop.
    Break: break_ => {},

    /// Returns from a function.
    Return: return_ => {},

    /// Returns a value from a function.
    ReturnVal: return_value => {
        /// The value to return.
        value: Readable,
    },

    // -----------------------------------------------------------------------------
    // ---------------------------------- VALUES------------------------------------
    // -----------------------------------------------------------------------------

    /// Allocates a new array.
    Array: array -> Writable => {
        /// The capacity of the array.
        capacity: u32,
    },

    /// Push a value to an array.
    Push: push -> Writable => {
        /// The value to push.
        value: Readable,
    },

    /// Allocates a new dictionary.
    Dict: dict -> Writable => {
        /// The capacity of the dictionary.
        capacity: u32,
    },

    /// Insert a value into a dictionary.
    Insert: insert -> Writable => {
        /// The key to insert.
        key: Readable,
        /// The value to insert.
        value: Readable,
    },

    /// Allocates a new argument set.
    Args: args -> Writable => {
        /// The capacity of the argument set.
        capacity: u32,
    },

    /// Pushes a value into an argument set.
    PushArg: push_arg -> Writable => {
        /// The value to insert.
        value: Readable,
        /// The span of the value.
        value_span: SpanId,
    },

    /// Inserts a named value into an argument set.
    InsertArg: insert_arg -> Writable => {
        /// The key to insert.
        key: PicoStr,
        /// The value to insert.
        value: Readable,
        /// The span of the value.
        value_span: SpanId,
    },

    /// Inserts a named value into an argument set.
    SpreadArg: spread_arg -> Writable => {
        /// The value to insert.
        value: Readable,
        /// The span of the value.
        value_span: SpanId,
    },

    /// Spreads this value into either:
    /// - An array.
    /// - A dictionary.
    /// - An argument set.
    Spread: spread -> Writable => {
        /// The value to spread.
        value: Readable,
    },

    // -----------------------------------------------------------------------------
    // ----------------------------- CONDITIONAL JUMPS -----------------------------
    // -----------------------------------------------------------------------------

    /// Enter a new scope with optional joining.
    Enter: enter -> Writable => {
        /// The length of the scope to enter.
        len: u32,
        /// Whether the scope produces content.
        content: bool,
    },

    /// Appends a marker to the current scope.
    PointerMarker: mark => {
        /// The marker to create.
        marker: Pointer,
    },

    /// Jump to a new instruction.
    Jump: jump => {
        /// The instruction to jump to.
        instruction: Pointer,
    },

    /// Jump to the top of the current scope.
    JumpTop: jump_top => {},

    /// Jump to a new instruction if the condition is true.
    JumpIf: jump_if => {
        /// The condition to check.
        condition: Readable,
        /// The instruction to jump to.
        instruction: Pointer,
    },

    JumpIfNot: jump_if_not => {
        /// The condition to check.
        condition: Readable,
        /// The instruction to jump to.
        instruction: Pointer,
    },

    /// Select one of two values based on a condition.
    Select: select -> Writable => {
        /// The condition to check.
        condition: Readable,
        /// The value to select if the condition is true.
        true_: Readable,
        /// The value to select if the condition is
        false_: Readable,
    },

    // -----------------------------------------------------------------------------
    // ----------------------------------- MATH ------------------------------------
    // -----------------------------------------------------------------------------

    /// Creates a new [`LrElem`].
    Delimited: delimited -> Writable => {
        /// The left delimiter.
        left: Readable,
        /// The body.
        body: Readable,
        /// The right delimiter.
        right: Readable,
    },

    /// Builds an [`AttachElem`].
    Attach: attach -> Writable => {
        /// The base value.
        base: Readable,
        /// The top supplement.
        top: Option<Readable>,
        /// The bottom supplement.
        bottom: Option<Readable>,
    },

    /// Builds a fraction.
    Frac: frac -> Writable => {
        /// The numerator.
        numerator: Readable,
        /// The denominator.
        denominator: Readable,
    },

    /// Builds a root.
    Root: root -> Writable => {
        /// The degree.
        degree: Option<Readable>,
        /// The radicand.
        radicand: Readable,
    },

    // -----------------------------------------------------------------------------
    // ---------------------------------- CONTENT ----------------------------------
    // -----------------------------------------------------------------------------

    /// Creates a new [`RefElem`] with a supplement.
    Ref: ref_ -> Writable => {
        /// The label of the reference.
        label: LabelId,

        /// The supplement.
        supplement: Readable,
    },

    /// Makes a value strong.
    Strong: strong -> Writable => {
        /// The value to make strong.
        value: Readable,
    },

    /// Makes a value emphasized.
    Emph: emph -> Writable => {
        /// The value to emphasize.
        value: Readable,
    },

    /// Makes a value into a heading.
    Heading: heading -> Writable => {
        /// The value to make into a heading.
        value: Readable,
        /// The level of the heading.
        level: NonZeroU32,
    },

    /// Makes a list item.
    ListItem: list_item -> Writable => {
        /// The value to make into a list item.
        value: Readable,
    },

    /// Makes an enum item.
    EnumItem: enum_item -> Writable => {
        /// The value to make into an enum item.
        value: Readable,
        /// The optional number of the enum item.
        number: Option<NonZeroU32>,
    },

    /// Markes a term.
    TermItem: term_item -> Writable => {
        /// The term to make into a term.
        term: Readable,
        /// The description of the term.
        description: Readable,
    },

    /// Makes an equation.
    Equation: equation -> Writable => {
        /// The value to make into an equation.
        value: Readable,
    },
}
