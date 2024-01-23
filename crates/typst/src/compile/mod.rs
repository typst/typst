use std::collections::HashSet;
use std::num::{NonZeroU16, NonZeroUsize};

use ecow::{eco_format, EcoString};
use smallvec::{smallvec, SmallVec};
use typst_syntax::{Span, Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, At, Hint, SourceResult};
use crate::engine::Engine;
use crate::eval::ops;
use crate::foundations::{
    array, call_method_mut, is_mutating_method, Args, Array, Content, Dict, Func,
    NativeElement, Recipe, Scopes, ShowableSelector, Str, Style, Styles, Transformation,
    Type, Unlabellable,
};
use crate::foundations::{calc::rem_euclid, IntoValue, Label, Smart, Value};
use crate::math::{AttachElem, EquationElem, FracElem, LrElem, RootElem};
use crate::model::{
    EmphElem, EnumItem, HeadingElem, ListItem, RefElem, StrongElem, Supplement, TermItem,
};

mod closure;
mod compiler;
mod destructure;
mod module;

pub use self::closure::*;
pub use self::compiler::*;
pub use self::destructure::*;
pub use self::module::*;

/// The maximum number of register slots.
pub const REGISTER_COUNT: u16 = 32;

#[derive(Clone, Debug, PartialEq, Hash, Default)]
pub struct RegisterTable {
    pub registers: [Value; REGISTER_COUNT as usize],
}

macro_rules! id {
    ($name:ident => $l:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(u16);

        impl $name {
            pub fn new(index: usize) -> Self {
                Self(index as u16)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($l, "{}"), self.0)
            }
        }
    };
    ($( $name:ident => $l:literal),* $(,)*) => {
        $( id!($name => $l); )*
    };
}

/// A type-checked register reference.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Register(u16);

impl std::fmt::Debug for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            write!(f, "NUL")
        } else {
            write!(f, "R{}", self.0)
        }
    }
}

impl Register {
    const NONE: Register = Register(0);

    pub fn new(index: usize) -> Self {
        Self(index as u16 + 1)
    }

    pub fn index(self) -> usize {
        self.0 as usize - 1
    }

    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

id! {
    IteratorId => "I",
    JmpLabel => "J",
    CallId => "Cal",
    ConstId => "Cnt",
    LocalId => "Loc",
    LabelId => "Lab",
    CapturedId => "Cap",
    PatternId => "P",
    AccessId => "Acc",
    ClosureId => "Clo",
    ArgumentId => "Arg",
    StringId => "S",
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum ModuleId {
    Global = 0,
    Math = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RegisterOrString {
    Register(Register),
    String(StringId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ScopeId(u16);

impl ScopeId {
    pub const SELF: Self = Self(0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum AssignOp {
    None = 0,
    Add,
    Mul,
    Sub,
    Div,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Instruction {
    /// A no-op.
    Noop = 0,
    /// Push a value into a register.
    Set {
        /// The register to push into.
        register: Register,
        /// The value to push.
        value: ConstId,
    },
    Copy {
        /// The register to copy from.
        source: Register,
        /// The register to copy to.
        target: Register,
    },
    /// Add two values.
    Add {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Subtract two values.
    Sub {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Multiply two values.
    Mul {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Divide two values.
    Div {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Modulo two values.
    Rem {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Logical and two values.
    And {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Logical or two values.
    Or {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Negate a value.
    Neg {
        /// The register to negate.
        target: Register,
    },
    /// Positive a value.
    Pos {
        /// The register to positive.
        target: Register,
    },
    /// Logical not a value.
    Not {
        /// The register to not.
        target: Register,
    },
    /// Whether a value is greater than another.
    Gt {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Whether a value is less than another.
    Lt {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Whether a value is greater than or equal to another.
    Geq {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Whether a value is less than or equal to another.
    Leq {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Compare two values for equality.
    Eq {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Compare two values for inequality.
    Neq {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Checks whether the left-hand side is contained inside of the right-hand side.
    In {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Checks whether the left-hand side is not contained inside of the right-hand side.
    NotIn {
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Push a new join group
    JoinGroup {
        /// The capacity hint of the join group.
        capacity: u16,
    },
    /// Pop a join group
    PopGroup {
        // The register in which to store the result.
        target: Register,
        /// Whether popping the group always produces a `content`.
        content: bool,
    },
    /// Join two values.
    Join {
        /// The value to add to the join group.
        value: Register,
    },
    /// Jump to a label.
    Jump {
        /// The label to jump to.
        label: JmpLabel,
    },
    /// Jump to a label if a condition is true.
    JumpIf {
        /// The condition register.
        condition: Register,
        /// The label to jump to.
        label: JmpLabel,
    },
    Label {
        /// The label defined at this location.
        label: JmpLabel,
    },
    /// Jump to a label if a condition is false.
    JumpIfNot {
        /// The condition register.
        condition: Register,
        /// The label to jump to.
        label: JmpLabel,
    },
    Select {
        /// The condition register.
        condition: Register,
        /// The left-hand side register.
        lhs: Register,
        /// The right-hand side register.
        rhs: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Call a function.
    Call { call: CallId },
    /// Return from a function.
    Return {
        /// The register to return from.
        register: Register,
    },
    /// Store a value in a local.
    Store {
        /// The scope to load from:
        /// - 0 is the current scope.
        /// - 1 is the parent scope.
        /// - 2 is the grandparent scope.
        /// - etc.
        scope: ScopeId,
        /// The local to store in.
        local: LocalId,
        /// The value to store.
        value: Register,
    },
    /// Load a local.
    Load {
        /// The scope to load from:
        /// - 0 is the current scope.
        /// - 1 is the parent scope.
        /// - 2 is the grandparent scope.
        /// - etc.
        scope: ScopeId,
        /// The local to load.
        local: LocalId,
        /// The register to load the result in.
        target: Register,
    },
    /// Load a captured local.
    /// Only used in closures.
    LoadCaptured {
        /// The capture to load.
        capture: CapturedId,
        /// The register to load the result in.
        target: Register,
    },
    LoadModule {
        /// The module to load from.
        module: ModuleId,

        /// The local to load.
        local: LocalId,

        /// The register to load the result in.
        target: Register,
    },
    /// Build a reference.
    Ref {
        /// The reference label.
        label: LabelId,

        /// The optional supplement register.
        supplement: Register,

        /// The register to store the result in.
        target: Register,
    },
    /// Build a strong value.
    Strong {
        /// The value to make strong.
        value: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build an emphasis value.
    Emph {
        /// The value to make emphasis.
        value: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a heading.
    Heading {
        /// The level of the heading.
        level: NonZeroU16,
        /// The body of the heading.
        body: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a list item.
    ListItem {
        /// The item in the list.
        item: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build an enum item.
    EnumItem {
        /// The number of the item.
        number: Option<u16>,
        /// The item in the enum.
        item: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a term item.
    TermItem {
        /// The term of the item.
        term: Register,
        /// The description of the item.
        description: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build an equation.
    Equation {
        /// Whether the equation is a block equation.
        block: bool,
        /// The body of the equation.
        body: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a display value.
    Display {
        /// The value to display.
        value: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a delimited value.
    Delimited {
        /// The left delimiter.
        left: Register,
        /// The body of the delimited value.
        body: Register,
        /// The right delimiter.
        right: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Attach a value to a value.
    Attach {
        /// The base value.
        base: Register,
        /// The top supplement.
        top: Register,
        /// The bottom supplement.
        bottom: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a fraction.
    Frac {
        /// The numerator.
        numerator: Register,
        /// The denominator.
        denominator: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Build a root.
    Root {
        /// The degree of the root.
        degree: Option<ConstId>,
        /// The radicand.
        radicand: Register,
        /// The register to store the result in.
        target: Register,
    },
    /// Instantiate an array.
    Array {
        /// The size (hint) of the array.
        size: u16,
        /// The register to store the result in.
        target: Register,
    },
    /// Push a value into an array.
    ArrayPush {
        /// The array to push into.
        array: Register,
        /// The value to push.
        value: Register,
    },
    /// Spread a value into an array.
    ArraySpread {
        /// The array to spread into.
        array: Register,
        /// The value to push spread.
        value: Register,
    },
    /// Instantiate a map.
    Dict {
        /// The size (hint) of the map.
        size: u16,
        /// The register to store the result in.
        target: Register,
    },
    /// Push a value into a map.
    DictInsert {
        /// The map to push into.
        dict: Register,
        /// The key to push.
        key: RegisterOrString,
        /// The value to push.
        value: Register,
    },
    /// Spread a value into a map.
    DictSpread {
        /// The map to spread into.
        dict: Register,
        /// The value to push spread.
        value: Register,
    },
    /// Build a new argument list.
    Args {
        /// The register to store the result in.
        target: Register,
    },
    /// Push a positional argument into an argument list.
    ArgsPush {
        /// The argument list to push into.
        args: Register,
        /// The value to push.
        value: Register,
    },
    /// Push a named argument into an argument list.
    ArgsInsert {
        /// The argument list to push into.
        args: Register,
        /// The key to push.
        key: StringId,
        /// The value to push.
        value: Register,
    },
    /// Spread a value into an argument list.
    ArgsSpread {
        /// The argument list to spread into.
        args: Register,
        /// The value to push spread.
        value: Register,
    },
    /// Access a field.
    FieldAccess {
        /// The value to access.
        value: Register,
        /// The field to access.
        field: StringId,
        /// The register to store the result in.
        target: Register,
    },
    /// Destructure a value.
    Destructure {
        /// The value to destructure.
        value: Register,
        /// The pattern to destructure into.
        pattern: PatternId,
    },
    /// Build an iterator.
    Iter {
        /// The value to iterate over.
        value: Register,
        /// The register to store the iterator in.
        iterator: IteratorId,
    },
    /// Get the next value from an iterator.
    Next {
        /// The iterator to get the next value from.
        iterator: IteratorId,
        /// The register to store the result in.
        target: Register,
        /// The label to jump to if the iterator is exhausted.
        exhausted: JmpLabel,
    },
    /// Load a closure.
    InstantiateClosure {
        /// The closure to instantiate.
        closure: ClosureId,
        /// The register to store the result in.
        target: Register,
    },
    LoadArg {
        /// The argument to load.
        arg: ArgumentId,
        /// The register to store the result in.
        target: Register,
    },
    SetRule {
        /// The target of the rule.
        target: Register,
        /// The arguments of the rule.
        args: Register,
        /// The register to store the result in.
        result: Register,
    },
    ShowRule {
        /// The selector of the rule.
        selector: Option<Register>,
        /// The transform of the rule.
        transform: Register,
        /// The register to store the result in.
        result: Register,
    },
    StylePush {
        /// The style to push.
        style: Register,
    },
    StylePop {
        /// The number of styling contexts to pop.
        depth: u16,
    },
    /// Enter a new scope with a fixed size.
    Enter { size: usize },
    /// Exit the current scope.
    Exit {},

    /// Include a module.
    Include {
        /// The source to include.
        source: Register,
        /// The target to store the result in.
        target: Register,
    },
    Assign {
        /// The target of the assignment.
        access: AccessId,
        /// The value to assign.
        value: Register,
        /// The operation to perform.
        op: AssignOp,
    },
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Call {
    /// The callee.
    callee: AccessPattern,
    /// The arguments.
    args: Register,
    /// The register to store the result in.
    target: Register,
    /// Whether it is called in a math context.
    math: bool,
    /// Whether there is a trailing comma.
    trailing_comma: bool,
}

impl Call {
    pub fn free(&self, compiler: &mut Compiler) {
        self.callee.free(compiler);
        compiler.free(self.args);
    }
}

#[derive(Debug)]
pub struct JoinContext {
    /// The styles to apply to the join group.
    styles: Option<Styles>,
    /// The content of the join group.
    elements: SmallVec<[(Value, Span); 4]>,
    /// The equivalent depth of the join group.
    depth: u16,
}

type Locals = SmallVec<[Value; 4]>;

pub struct Executor<'a> {
    /// The current register table.
    pub registers: RegisterTable,
    /// The locals used in the instruction set.
    pub locals: Locals,
    /// The scopes used in the instruction set.
    pub scopes: Scopes<'a>,
    /// The instructions to execute.
    pub instructions: &'a [Instruction],
    /// The labels in the instruction set.
    pub labels: &'a [usize],
    /// The calls in the instruction set.
    pub calls: &'a [Call],
    /// The constants in the instruction set.
    pub constants: &'a [Value],
    /// The arguments used in the instruction set.
    pub arguments: &'a [Value],
    /// The closures in the instruction set.
    pub closures: &'a [CompiledClosure],
    /// The strings in the instruction set.
    pub strings: &'a [EcoString],
    /// The captured locals used in the instruction set.
    pub captured: &'a [Value],
    /// The content labels in the instruction set.
    pub content_labels: &'a [Label],
    /// The stack of scopes (locals)
    pub scope_stack: SmallVec<[Locals; 4]>,
    /// Join contexts.
    pub join_contexts: SmallVec<[JoinContext; 4]>,
    /// The patterns in the instruction set.
    pub patterns: &'a [Pattern],
    /// The spans in the instruction set.
    pub spans: &'a [Span],
    /// The default output in case of no return.
    pub output: Register,
    /// The iterator stack.
    pub iterators: SmallVec<[Box<dyn Iterator<Item = Value>>; 4]>,
    /// The access patterns in the instruction set.
    pub accesses: &'a [AccessPattern],
}

#[derive(Clone, Debug, PartialEq, Hash)]
enum ControlFlow {
    /// Continue execution.
    Continue,
    /// Stop execution.
    Stop(Value),
    /// Jump to a label.
    Jump(JmpLabel),
}

impl Executor<'_> {
    pub fn eval(&mut self, engine: &mut Engine) -> SourceResult<Value> {
        // The instruction pointer.
        let mut ip = 0;
        while ip < self.instructions.len() {
            let isr = self.instructions[ip];
            match self.eval_one(engine, ip, isr)? {
                ControlFlow::Continue => ip += 1,
                ControlFlow::Stop(value) => return Ok(value),
                ControlFlow::Jump(label) => {
                    ip = self.labels[label.0 as usize];
                }
            }
        }

        Ok(self.get(self.output).clone())
    }

    fn const_(&self, id: ConstId) -> &Value {
        self.constants.get(id.0 as usize).expect("const is out of bounds")
    }

    fn str(&self, id: StringId) -> &EcoString {
        self.strings.get(id.0 as usize).expect("string is out of bounds")
    }

    fn label(&self, label: LabelId) -> Label {
        self.content_labels
            .get(label.0 as usize)
            .expect("label is out of bounds")
            .clone()
    }

    fn call(&self, call: CallId) -> &Call {
        self.calls.get(call.0 as usize).expect("call is out of bounds")
    }

    /// Get a register.
    fn get(&self, register: Register) -> &Value {
        if register.is_none() {
            return &Value::None;
        }

        self.registers
            .registers
            .get(register.0 as usize - 1)
            .expect("register is out of bounds")
    }

    fn get_mut(&mut self, register: Register) -> &mut Value {
        if register.is_none() {
            unreachable!("improperly used NONE register");
        }

        self.registers
            .registers
            .get_mut(register.0 as usize - 1)
            .expect("register is out of bounds")
    }

    fn set(&mut self, register: Register, value: Value) {
        if register.is_none() {
            return;
        }

        let reg = self
            .registers
            .registers
            .get_mut(register.0 as usize - 1)
            .expect("register is out of bounds");

        *reg = value;
    }

    fn captured(&self, captured: CapturedId) -> &Value {
        self.captured
            .get(captured.0 as usize)
            .expect("captured is out of bounds")
    }

    fn eval_one(
        &mut self,
        engine: &mut Engine,
        ip: usize,
        isr: Instruction,
    ) -> SourceResult<ControlFlow> {
        match isr {
            Instruction::Noop | Instruction::Label { .. } => self.eval_noop(),
            Instruction::Set { register, value } => {
                let value = self.const_(value).clone();
                self.set(register, value);
            }
            Instruction::Copy { source, target } => {
                let value = self.get(source).clone();
                self.set(target, value);
            }
            Instruction::Add { lhs, rhs, target } => self.add(ip, lhs, rhs, target)?,
            Instruction::Sub { lhs, rhs, target } => self.sub(ip, lhs, rhs, target)?,
            Instruction::Mul { lhs, rhs, target } => self.mul(ip, lhs, rhs, target)?,
            Instruction::Div { lhs, rhs, target } => self.div(ip, lhs, rhs, target)?,
            Instruction::Rem { lhs, rhs, target } => self.rem(ip, lhs, rhs, target)?,
            Instruction::Neg { target } => self.neg(ip, target)?,
            Instruction::Gt { lhs, rhs, target } => self.gt(ip, lhs, rhs, target)?,
            Instruction::Lt { lhs, rhs, target } => self.lt(ip, lhs, rhs, target)?,
            Instruction::Geq { lhs, rhs, target } => self.ge(ip, lhs, rhs, target)?,
            Instruction::Leq { lhs, rhs, target } => self.le(ip, lhs, rhs, target)?,
            Instruction::Eq { lhs, rhs, target } => self.eq(ip, lhs, rhs, target)?,
            Instruction::Neq { lhs, rhs, target } => self.neq(ip, lhs, rhs, target)?,
            Instruction::Jump { label } => return Ok(ControlFlow::Jump(label)),
            Instruction::JumpIf { condition, label } => {
                return self.jump_if(ip, condition, label, true)
            }
            Instruction::JumpIfNot { condition, label } => {
                return self.jump_if(ip, condition, label, false)
            }
            Instruction::Return { register } => return self.ret(ip, register),
            Instruction::Ref { label, supplement, target } => {
                self.ref_(ip, label, supplement, target)?
            }
            Instruction::Store { scope, local, value } => self.store(ip, scope, local, value)?,
            Instruction::Load { local, scope, target } => self.load(scope, local, target)?,
            Instruction::LoadModule { module, local, target } => {
                let Some(library) = self.scopes.base else {
                    bail!(self.spans[ip], "library scope is missing");
                };

                let value = match module {
                    ModuleId::Global => {
                        library.global.field_by_id(local.0 as usize).at(self.spans[ip])?
                    }
                    ModuleId::Math => {
                        library.math.field_by_id(local.0 as usize).at(self.spans[ip])?
                    }
                };

                self.set(target, value.clone());
            }
            Instruction::Strong { value, target } => {
                let value = self.get(value).clone();
                let strong = StrongElem::new(value.cast().at(self.spans[ip])?)
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, strong.into_value());
            }
            Instruction::Emph { value, target } => {
                let value = self.get(value).clone();
                let emph = EmphElem::new(value.cast().at(self.spans[ip])?)
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, emph.into_value());
            }
            Instruction::Heading { level, body, target } => {
                let body = self.get(body).clone();
                let heading = HeadingElem::new(body.cast().at(self.spans[ip])?)
                    .with_level(NonZeroUsize::from(level))
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, heading.into_value());
            }
            Instruction::ListItem { item, target } => {
                let item = self.get(item).clone();
                let list_item = ListItem::new(item.cast().at(self.spans[ip])?)
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, list_item.into_value());
            }
            Instruction::EnumItem { number, item, target } => {
                let item = self.get(item).clone();
                let enum_item = EnumItem::new(item.cast().at(self.spans[ip])?)
                    .with_number(number.map(|u| u as usize))
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, enum_item.into_value());
            }
            Instruction::TermItem { term, description, target } => {
                let term = self.get(term).clone();
                let description = self.get(description).clone();
                let term_item = TermItem::new(
                    term.cast().at(self.spans[ip])?,
                    description.cast().at(self.spans[ip])?,
                )
                .pack()
                .spanned(self.spans[ip]);
                self.set(target, term_item.into_value());
            }
            Instruction::Equation { block, body, target } => {
                let body = self.get(body).clone();
                let equation = EquationElem::new(body.cast().at(self.spans[ip])?)
                    .with_block(block)
                    .pack()
                    .spanned(self.spans[ip]);
                self.set(target, equation.into_value());
            }
            Instruction::Display { value, target } => {
                let value = self.get(value).clone().display();

                self.set(target, value.into_value());
            }
            Instruction::Delimited { left, body, right, target } => {
                let left: Content = self.get(left).clone().cast().at(self.spans[ip])?;
                let body: Content = self.get(body).clone().cast().at(self.spans[ip])?;
                let right: Content = self.get(right).clone().cast().at(self.spans[ip])?;

                let delimited =
                    LrElem::new(left + body + right).pack().spanned(self.spans[ip]);

                self.set(target, delimited.into_value());
            }
            Instruction::Attach { base, top, bottom, target } => {
                let base: Content = self.get(base).clone().cast().at(self.spans[ip])?;

                let mut attach = AttachElem::new(base);

                if !top.is_none() {
                    let top: Content = self.get(top).clone().cast().at(self.spans[ip])?;
                    attach.push_t(Some(top));
                }

                if !bottom.is_none() {
                    let bottom: Content =
                        self.get(bottom).clone().cast().at(self.spans[ip])?;
                    attach.push_b(Some(bottom));
                }

                self.set(target, attach.pack().spanned(self.spans[ip]).into_value());
            }
            Instruction::Frac { numerator, denominator, target } => {
                let numerator: Content =
                    self.get(numerator).clone().cast().at(self.spans[ip])?;
                let denominator: Content =
                    self.get(denominator).clone().cast().at(self.spans[ip])?;

                let frac =
                    FracElem::new(numerator, denominator).pack().spanned(self.spans[ip]);

                self.set(target, frac.into_value());
            }
            Instruction::Root { degree, radicand, target } => {
                let radicand: Content =
                    self.get(radicand).clone().cast().at(self.spans[ip])?;

                let mut root = RootElem::new(radicand);

                if let Some(degree) = degree {
                    let degree: Content =
                        self.const_(degree).clone().cast().at(self.spans[ip])?;
                    root.push_index(Some(degree))
                }

                self.set(target, root.pack().spanned(self.spans[ip]).into_value());
            }
            Instruction::And { lhs, rhs, target } => {
                let lhs = self.get(lhs).clone();
                let rhs = self.get(rhs).clone();
                let value = ops::and(lhs, rhs).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::Or { lhs, rhs, target } => {
                let lhs = self.get(lhs).clone();
                let rhs = self.get(rhs).clone();
                let value = ops::or(lhs, rhs).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::Pos { target } => {
                let value = self.get(target).clone();
                let value = ops::pos(value).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::Not { target } => {
                let value = self.get(target).clone();
                let value = ops::not(value).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::In { lhs, rhs, target } => {
                let lhs = self.get(lhs).clone();
                let rhs = self.get(rhs).clone();
                let value = ops::in_(lhs, rhs).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::NotIn { lhs, rhs, target } => {
                let lhs = self.get(lhs).clone();
                let rhs = self.get(rhs).clone();
                let value = ops::not_in(lhs, rhs).at(self.spans[ip])?;
                self.set(target, value);
            }
            Instruction::Array { size, target } => {
                let array = Array::with_capacity(size as usize);
                self.set(target, array.into_value());
            }
            Instruction::ArrayPush { array, value } => {
                let span = self.spans[ip];
                let value = self.get(value).clone();
                let array = self.get_mut(array);

                let Value::Array(array) = array else {
                    bail!(span, "expected array, found {}", array.ty().short_name());
                };

                array.push(value);
            }
            Instruction::ArraySpread { array, value } => {
                let span = self.spans[ip];
                let value = self.get(value).clone();
                let array = self.get_mut(array);

                let Value::Array(array) = array else {
                    bail!(span, "expected array, found {}", array.ty().short_name());
                };

                match value {
                    Value::None => {}
                    Value::Array(value) => {
                        array.extend(value.into_iter());
                    }
                    _ => bail!(span, "expected array, found {}", value.ty().short_name()),
                }
            }
            Instruction::Dict { size, target } => {
                let dict = Dict::with_capacity(size as usize);
                self.set(target, dict.into_value());
            }
            Instruction::DictInsert { dict, key, value } => {
                let span = self.spans[ip];
                let key = match key {
                    RegisterOrString::Register(key) => {
                        self.get(key).clone().cast::<Str>().at(span)?
                    }
                    RegisterOrString::String(key) => Str::from(self.str(key).clone()),
                };

                let value = self.get(value).clone();
                let dict = self.get_mut(dict);

                let Value::Dict(dict) = dict else {
                    bail!(span, "expected dict, found {}", dict.ty().short_name());
                };

                dict.insert(key, value);
            }
            Instruction::DictSpread { dict, value } => {
                let span = self.spans[ip];
                let value = self.get(value).clone();
                let dict = self.get_mut(dict);

                let Value::Dict(dict) = dict else {
                    bail!(span, "expected dict, found {}", dict.ty().short_name());
                };

                match value {
                    Value::None => {}
                    Value::Dict(value) => {
                        dict.extend(value.into_iter());
                    }
                    _ => bail!(span, "expected dict, found {}", value.ty().short_name()),
                }
            }
            Instruction::FieldAccess { value, field, target } => {
                let value = self.get(value);
                let field = self.str(field);

                // Check for associated functions.
                let value = if let Some(assoc) = value.ty().scope().get(field) {
                    let Value::Func(method) = assoc else {
                        bail!(
                            self.spans[ip],
                            "expected function, found {}",
                            assoc.ty().short_name()
                        );
                    };

                    Value::Func(Func::method(value.clone(), method.clone()))
                } else {
                    value.field(&field).at(self.spans[ip])?
                };

                self.set(target, value);
            }
            Instruction::Select { condition, lhs, rhs, target } => {
                let condition = self.get(condition).clone();
                if condition.cast::<bool>().at(self.spans[ip])? {
                    let lhs = self.get(lhs).clone();
                    self.set(target, lhs);
                } else {
                    let rhs = self.get(rhs).clone();
                    self.set(target, rhs);
                }
            }
            Instruction::Args { target } => {
                let args = Args::new(self.spans[ip], std::iter::empty::<Value>());
                self.set(target, args.into_value());
            }
            Instruction::ArgsPush { args, value } => {
                let span = self.spans[ip];
                let value = self.get(value).clone();
                let args = self.get_mut(args);

                let Value::Args(args) = args else {
                    bail!(span, "expected args, found {}", args.ty().short_name());
                };

                args.push(span, value);
            }
            Instruction::ArgsInsert { args, key, value } => {
                let span = self.spans[ip];
                let key = self.str(key).clone();
                let value = self.get(value).clone();
                let args = self.get_mut(args);

                let Value::Args(args) = args else {
                    bail!(span, "expected args, found {}", args.ty().short_name());
                };

                args.insert(span, Str::from(key), value);
            }
            Instruction::ArgsSpread { args, value } => {
                let span = self.spans[ip];
                let value = self.get(value).clone();
                let args = self.get_mut(args);

                let Value::Args(args) = args else {
                    bail!(span, "expected args, found {}", args.ty().short_name());
                };

                match value {
                    Value::None => {}
                    Value::Array(array) => {
                        args.extend(array.into_iter());
                    }
                    Value::Dict(dict) => {
                        args.extend(dict.into_iter());
                    }
                    Value::Args(value) => {
                        args.chain(value);
                    }
                    _ => bail!(
                        span,
                        "expected none, array, dict, or args and found {}",
                        value.ty().short_name()
                    ),
                }
            }
            Instruction::Call { call } => {
                let span = self.spans[ip];
                let call = self.call(call).clone();

                let args = match self.get(call.args) {
                    Value::Args(args) => args.clone(),
                    Value::None => Args::new(span, std::iter::empty::<Value>()),
                    other => {
                        bail!(
                            span,
                            "expected `arguments`, found {}",
                            other.ty().short_name()
                        )
                    }
                };

                match &call.callee {
                    // Special case to handle mutable methods.
                    AccessPattern::Chained(rest, last) if is_mutating_method(last) => {
                        let callee = self.access(ip, rest)?;
                        let res = call_method_mut(callee, last, args, span)?.clone();
                        self.set(call.target, res);
                    }
                    other => {
                        let callee = self.access(ip, &other)?;

                        let callee = match callee {
                            Value::Func(callee) => callee.clone(),
                            Value::Type(type_) => type_.constructor().at(span)?,
                            _ => bail!(
                                span,
                                "expected function, found {}",
                                callee.ty().short_name()
                            ),
                        };

                        let res = callee.call(engine, args)?;
                        self.set(call.target, res);
                    }
                };
            }
            Instruction::LoadCaptured { capture, target } => {
                let value = self.captured(capture).clone();
                self.set(target, value);
            }
            Instruction::InstantiateClosure { closure, target } => {
                let closure = self
                    .closures
                    .get(closure.0 as usize)
                    .expect("closure is out of bounds");

                let closure = Closure::instantiate(self, closure)?;

                self.set(target, Value::Func(Func::from(closure)))
            }
            Instruction::LoadArg { arg, target } => {
                let value = self.arguments[arg.0 as usize].clone();
                self.set(target, value);
            }
            Instruction::SetRule { target, args, result } => {
                let target = self
                    .get(target)
                    .clone()
                    .cast::<Func>()
                    .and_then(|func| {
                        func.element().ok_or_else(|| {
                            "only element functions can be used in set rules".into()
                        })
                    })
                    .at(self.spans[ip])?;

                let args = self.get(args);
                let Value::Args(args) = args.clone() else {
                    bail!(
                        self.spans[ip],
                        "expected args, found {}",
                        args.ty().short_name()
                    );
                };

                self.set(
                    result,
                    target.set(engine, args)?.spanned(self.spans[ip]).into_value(),
                );
            }
            Instruction::ShowRule { selector, transform, result } => {
                let selector = if let Some(selector) = selector {
                    let selector = self.get(selector).clone();
                    Some(selector.cast::<ShowableSelector>().at(self.spans[ip])?)
                } else {
                    None
                };

                let transform = self
                    .get(transform)
                    .clone()
                    .cast::<Transformation>()
                    .at(self.spans[ip])?;

                let mut out = Styles::new();
                out.apply_one(Style::Recipe(Recipe {
                    span: self.spans[ip],
                    selector: selector.map(|selector| selector.0),
                    transform,
                }));

                self.set(result, out.into_value());
            }
            Instruction::StylePush { style } => {
                let styles =
                    self.get(style).clone().cast::<Styles>().at(self.spans[ip])?;

                if let Some(context) = self.join_contexts.last_mut() {
                    if context.styles.is_some() && context.elements.is_empty() {
                        context.depth += 1;
                        context.styles.as_mut().unwrap().apply_slice(styles.as_slice());
                    } else {
                        let join_context = JoinContext {
                            styles: Some(styles),
                            elements: SmallVec::new(),
                            depth: 1,
                        };

                        self.join_contexts.push(join_context);
                    }
                } else {
                    let join_context = JoinContext {
                        styles: Some(styles),
                        elements: SmallVec::new(),
                        depth: 1,
                    };

                    self.join_contexts.push(join_context);
                }
            }
            Instruction::StylePop { depth } => {
                // Pop each style, and turns its content into a styled sequence
                // Appending it to the parent join context.

                let mut i: u16 = 0;
                while i < depth {
                    let join_context = self.join_contexts.pop().unwrap();
                    i += join_context.depth;

                    let span = join_context
                        .elements
                        .last()
                        .map(|(_, span)| *span)
                        .unwrap_or_else(|| self.spans[ip]);

                    let Some(styles) = join_context.styles else {
                        bail!(span, "style pop without style")
                    };

                    let elems =
                        join_context.elements.into_iter().map(|(elem, _)| elem.display());
                    let out =
                        Content::sequence(elems).styled_with_map(styles).into_value();
                    self.join_contexts.last_mut().unwrap().elements.push((out, span));
                }
            }
            Instruction::JoinGroup { capacity } => {
                let join_context = JoinContext {
                    styles: None,
                    elements: SmallVec::with_capacity(capacity as usize),
                    depth: 1,
                };

                self.join_contexts.push(join_context);
            }
            Instruction::PopGroup { target, content } => {
                let Some(mut group) = self.join_contexts.pop() else {
                    bail!(
                        self.spans[ip],
                        "attempted to pop join group when none are active"
                    );
                };

                // We we have some style, then we know that we need to apply it to the group.
                let out = if let Some(styles) = group.styles {
                    // If there's only one element in the group, then we can just apply the style
                    // to the element.
                    if group.elements.is_empty() {
                        Value::None
                    } else if group.elements.len() == 1 {
                        group
                            .elements
                            .pop()
                            .unwrap()
                            .0
                            .display()
                            .styled_with_map(styles)
                            .into_value()
                    } else {
                        let elems = group
                            .elements
                            .into_iter()
                            .map(|(elem, span)| elem.display().spanned(span));
                        Content::sequence(elems).styled_with_map(styles).into_value()
                    }
                } else {
                    let mut elems = group.elements.into_iter();
                    let mut out =
                        elems.next().map(|(elem, _)| elem).unwrap_or_else(|| Value::None);
                    for (elem, span) in elems {
                        out = ops::join(out, elem).at(span)?;
                    }

                    out
                };

                if out.is_none() && content {
                    self.set(target, Content::sequence(std::iter::empty()).into_value());
                } else {
                    self.set(target, out);
                }
            }
            Instruction::Join { value } => {
                let value = self.get(value).clone();
                let Some(join_context) = self.join_contexts.last_mut() else {
                    bail!(
                        self.spans[ip],
                        "attempted to join when no join group is active"
                    )
                };

                if !value.is_none() {
                    if let Value::Label(label) = &value {
                        if let Some(elem) = join_context
                            .elements
                            .iter_mut()
                            .rev()
                            .filter_map(|(elem, _)| {
                                if let Value::Content(cnt) = elem {
                                    Some(cnt)
                                } else {
                                    None
                                }
                            })
                            .find(|node| !node.can::<dyn Unlabellable>())
                        {
                            *elem = std::mem::take(elem).labelled(*label);
                        }
                    } else {
                        join_context.elements.push((value, self.spans[ip]));
                    }
                }
            }
            Instruction::Enter { size } => {
                let mut locals = smallvec![Value::None; size as usize];
                std::mem::swap(&mut self.locals, &mut locals);
                self.scope_stack.push(locals);
            }
            Instruction::Exit {} => {
                let mut locals = self.scope_stack.pop().unwrap();
                std::mem::swap(&mut self.locals, &mut locals);
            }

            Instruction::Iter { value, iterator } => {
                let value = self.get(value).clone();
                let iter: Box<dyn Iterator<Item = Value>> = match value {
                    Value::Str(string) => {
                        let vec: Vec<Value> = string
                            .graphemes(true)
                            .map(|s| Value::Str(s.into()))
                            .collect();
                        Box::new(vec.into_iter())
                    }
                    Value::Array(array) => Box::new(array.into_iter()),
                    Value::Dict(dict) => {
                        Box::new(dict.into_iter().map(|(key, value)| {
                            array![key.into_value(), value].into_value()
                        }))
                    }
                    value => bail!(
                        self.spans[ip],
                        "cannot iterate over {}",
                        value.ty().short_name()
                    ),
                };

                debug_assert!(self.iterators.len() == iterator.0 as usize);
                self.iterators.push(iter);
            }
            Instruction::Next { iterator, target, exhausted } => {
                let iter = self.iterators.get_mut(iterator.0 as usize).unwrap();
                let value = iter.next();
                if let Some(value) = value {
                    self.set(target, value);
                } else {
                    // Remove the iterator
                    self.iterators.pop();

                    // Set the value to none
                    self.set(target, Value::None);

                    // Go to the exhausted label
                    return Ok(ControlFlow::Jump(exhausted));
                }
            }
            Instruction::Include { source, target } => {
                let source = self.get(source).clone();
                let module = import(engine, self.spans[ip], source)?;

                self.set(target, module.content().into_value());
            }
            Instruction::Destructure { value, pattern } => {
                let value = self.get(value).clone();
                let pattern = self.patterns[pattern.0 as usize].clone();

                match &pattern.kind {
                    PatternKind::Single(single) => match single {
                        // We do nothing
                        PatternItem::Placeholder(_) => {}
                        PatternItem::Simple(_, local, _) => {
                            *self.access(ip, local)? = value;
                        }
                        PatternItem::Named(span, _, _) => {
                            bail!(
                                *span,
                                "cannot destructure {} with named pattern",
                                value.ty().short_name()
                            )
                        }
                        PatternItem::Spread(span, _)
                        | PatternItem::SpreadDiscard(span) => bail!(
                            *span,
                            "cannot destructure {} with spread",
                            value.ty().short_name()
                        ),
                    },
                    PatternKind::Tuple(tuple) => match value {
                        Value::Array(array) => destructure_array(self, ip, array, tuple)?,
                        Value::Dict(dict) => destructure_dict(self, ip, dict, tuple)?,
                        other => bail!(
                            self.spans[ip],
                            "cannot destructure {}",
                            other.ty().short_name()
                        ),
                    },
                }
            }
            Instruction::Assign { access, value, op } => {
                let span = self.spans[ip];
                let access = &self.accesses[access.0 as usize];
                let rhs = self.get(value).clone();
                let value = self.access(ip, access)?;
                let lhs = std::mem::take(value);

                match op {
                    AssignOp::None => *value = rhs,
                    AssignOp::Add => *value = ops::add(lhs, rhs).at(span)?,
                    AssignOp::Mul => *value = ops::mul(lhs, rhs).at(span)?,
                    AssignOp::Sub => *value = ops::sub(lhs, rhs).at(span)?,
                    AssignOp::Div => *value = ops::div(lhs, rhs).at(span)?,
                }
            }
        }

        Ok(ControlFlow::Continue)
    }

    fn eval_noop(&mut self) {
        // Do nothing.
    }

    fn access(&mut self, ip: usize, pattern: &AccessPattern) -> SourceResult<&mut Value> {
        let span = self.spans[ip];
        match pattern {
            AccessPattern::Register(register) => {
                if register.is_none() {
                    bail!(span, "cannot access NONE register");
                }

                let value = self.get_mut(*register);
                Ok(value)
            }
            AccessPattern::Local(scope, local) => {
                let Some(value) = self.local_mut(*scope, *local) else {
                    bail!(span, "cannot load remote local: out of bounds")
                };

                Ok(value)
            }
            AccessPattern::Chained(lhs, rhs) => {
                let value = self.access(ip, lhs)?;
                match value {
                    Value::Dict(dict) => dict.at_mut(rhs).at(span),
                    value => {
                        let ty = value.ty();
                        if matches!(
                            value, // those types have their own field getters
                            Value::Symbol(_)
                                | Value::Content(_)
                                | Value::Module(_)
                                | Value::Func(_)
                        ) {
                            bail!(span, "cannot mutate fields on {ty}");
                        } else if crate::foundations::fields_on(ty).is_empty() {
                            bail!(span, "{ty} does not have accessible fields");
                        } else {
                            // type supports static fields, which don't yet have
                            // setters
                            Err(eco_format!("fields on {ty} are not yet mutable"))
                                .hint(eco_format!(
                                    "try creating a new {ty} with the updated field value instead"
                                ))
                                .at(span)
                        }
                    }
                }
            }
            AccessPattern::AccessorMethod(accessor, call_id, name) => {
                let call = self.call(*call_id).clone();
                let args = match self.get(call.args) {
                    Value::Args(args) => args.clone(),
                    Value::None => Args::new(span, std::iter::empty::<Value>()),
                    other => {
                        bail!(
                            span,
                            "expected `arguments`, found {}",
                            other.ty().short_name(),
                        )
                    }
                };

                let callee = self.access(ip, accessor)?;
                call_method_access(callee, name, args, span)
            }
        }
    }

    fn add(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::add(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn sub(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::sub(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn mul(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::mul(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn div(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::div(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn rem(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();

        let lhs = lhs.cast().at(self.spans[ip])?;
        let rhs = Spanned::new(rhs.cast().at(self.spans[ip])?, self.spans[ip]);
        let value = rem_euclid(lhs, rhs)?;
        self.set(target, value.into_value());
        Ok(())
    }

    fn neg(&mut self, ip: usize, target: Register) -> SourceResult<()> {
        let value = self.get(target).clone();
        let value = ops::neg(value).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn gt(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::gt(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn ge(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::geq(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn lt(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::lt(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn le(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::leq(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn eq(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::eq(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn neq(
        &mut self,
        ip: usize,
        lhs: Register,
        rhs: Register,
        target: Register,
    ) -> SourceResult<()> {
        let lhs = self.get(lhs).clone();
        let rhs = self.get(rhs).clone();
        let value = ops::neq(lhs, rhs).at(self.spans[ip])?;
        self.set(target, value);
        Ok(())
    }

    fn jump_if(
        &mut self,
        ip: usize,
        condition: Register,
        label: JmpLabel,
        jump: bool,
    ) -> SourceResult<ControlFlow> {
        let condition = self.get(condition).clone();
        let condition = condition.cast::<bool>().at(self.spans[ip])?;
        if condition == jump {
            return Ok(ControlFlow::Jump(label));
        }

        Ok(ControlFlow::Continue)
    }

    fn ret(&mut self, _: usize, register: Register) -> SourceResult<ControlFlow> {
        let value = self.get(register).clone();
        Ok(ControlFlow::Stop(value))
    }

    fn ref_(
        &mut self,
        ip: usize,
        label: LabelId,
        supplement: Register,
        target: Register,
    ) -> SourceResult<()> {
        let label = self.label(label);
        let mut ref_ = RefElem::new(label);
        if !supplement.is_none() {
            let supplement = self.get(supplement).clone();
            ref_.push_supplement(Smart::Custom(Some(Supplement::Content(
                supplement.clone().cast().at(self.spans[ip])?,
            ))));
        }

        self.set(target, ref_.pack().into_value());

        Ok(())
    }

    fn store(&mut self, _: usize, scope: ScopeId, local: LocalId, value: Register) -> SourceResult<()> {
        let value = self.get(value).clone();
        let loc = if scope == ScopeId::SELF {
            self
                .locals
                .get_mut(local.0 as usize)
                .expect("cannot store local: out of bounds")
        } else {
            self.scope_stack
                .iter_mut()
                .rev()
                .nth(scope.0 as usize - 1)
                .and_then(|locals| locals.get_mut(local.0 as usize))
                .expect("cannot load local: out of bounds")
        };

        *loc = value;
        Ok(())
    }

    fn load(&mut self, scope: ScopeId, local: LocalId, target: Register) -> SourceResult<()> {
        let value = if scope == ScopeId::SELF {
            self
                .locals
                .get(local.0 as usize)
                .expect("cannot load local: out of bounds")
                .clone()
        } else {
            self.scope_stack
                .iter()
                .rev()
                .nth(scope.0 as usize - 1)
                .and_then(|locals| locals.get(local.0 as usize))
                .expect("cannot load local: out of bounds")
                .clone()
        };
        self.set(target, value);
        Ok(())
    }

    fn local(&self, scope: ScopeId, local: LocalId) -> Option<&Value> {
        if scope == ScopeId::SELF {
            self
                .locals
                .get(local.0 as usize)
        } else {
            self.scope_stack
                .iter()
                .rev()
                .nth(scope.0 as usize - 1)
                .and_then(|locals| locals.get(local.0 as usize))
        }
    }

    fn local_mut(&mut self, scope: ScopeId, local: LocalId) -> Option<&mut Value> {
        if scope == ScopeId::SELF {
            self
                .locals
                .get_mut(local.0 as usize)
        } else {
            self.scope_stack
                .iter_mut()
                .rev()
                .nth(scope.0 as usize - 1)
                .and_then(|locals| locals.get_mut(local.0 as usize))
        }
    }
}

/// Call an accessor method on a value.
pub(crate) fn call_method_access<'a>(
    value: &'a mut Value,
    method: &str,
    mut args: Args,
    span: Span,
) -> SourceResult<&'a mut Value> {
    let ty = value.ty();
    let missing = || Err(missing_method(ty, method)).at(span);

    let slot = match value {
        Value::Array(array) => match method {
            "first" => array.first_mut().at(span)?,
            "last" => array.last_mut().at(span)?,
            "at" => array.at_mut(args.expect("index")?).at(span)?,
            _ => return missing(),
        },
        Value::Dict(dict) => match method {
            "at" => dict.at_mut(&args.expect::<Str>("key")?).at(span)?,
            _ => return missing(),
        },
        _ => return missing(),
    };

    args.finish()?;
    Ok(slot)
}

/// The missing method error message.
#[cold]
fn missing_method(ty: Type, method: &str) -> String {
    format!("type {ty} has no method `{method}`")
}

fn destructure_array(
    executor: &mut Executor,
    ip: usize,
    value: Array,
    tuple: &[PatternItem],
) -> SourceResult<()> {
    let mut i = 0;
    let len = value.as_slice().len();
    for p in tuple {
        match p {
            PatternItem::Named(span, _, _) => {
                bail!(*span, "cannot destructure array with named pattern")
            }
            PatternItem::Placeholder(span) => {
                if i < len {
                    i += 1
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::Simple(span, local, _) => {
                if i < len {
                    *executor.access(ip, local)? = value.as_slice()[i].clone();
                    i += 1;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::Spread(span, local) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                if let (Some(sink_size), Some(sink)) = (sink_size, sink) {
                    *executor.access(ip, local)? = Value::Array(sink.into());
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::SpreadDiscard(span) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                if let (Some(sink_size), Some(_)) = (sink_size, sink) {
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
        }
    }

    Ok(())
}

fn destructure_dict(
    executor: &mut Executor,
    ip: usize,
    dict: Dict,
    tuple: &[PatternItem],
) -> SourceResult<()> {
    let mut sink = None;
    let mut used = HashSet::new();

    for p in tuple {
        match p {
            PatternItem::Simple(span, local, name) => {
                let v = dict.get(&name).at(*span)?;
                *executor.access(ip, local)? = v.clone();
                used.insert(name.clone());
            }
            PatternItem::Placeholder(_) => {}
            PatternItem::Spread(span, local) => sink = Some((*span, Some(local))),
            PatternItem::SpreadDiscard(span) => sink = Some((*span, None)),
            PatternItem::Named(span, access, name) => {
                let access = executor.access(ip, access)?;
                let v = dict.get(&name).at(*span)?;
                *access = v.clone();
                used.insert(name.clone());
            }
        }
    }

    if let Some((_, local)) = sink {
        if let Some(local) = local {
            let mut sink = Dict::new();
            for (key, value) in dict {
                if !used.contains(key.as_str()) {
                    sink.insert(key, value);
                }
            }

            *executor.access(ip, local)? = Value::Dict(sink);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use comemo::{Prehashed, Track, Tracked};
    use typst_syntax::{ast, parse, FileId, Source};

    use crate::{
        compile::Compile,
        diag::{FileResult, SourceDiagnostic},
        engine::{Engine, Route},
        eval::Tracer,
        foundations::{Bytes, Datetime},
        introspection::{Introspector, Locator},
        text::{Font, FontBook},
        Library, World,
    };

    struct BenchWorld {
        library: Prehashed<Library>,
        book: Prehashed<FontBook>,
        font: Font,
        source: Source,
    }
    const FONT: &[u8] = include_bytes!("../../../../assets/fonts/LinLibertine_R.ttf");

    const CODE: &str = r#"
    #for (a, b) in ((1, "a"), (2, "b"), (3, "c")) {
        repr((a, b))
    }
    "#;

    #[test]
    fn test_other() {
        let world = BenchWorld::new();
        let mut tracer = Tracer::new();
        let route = Route::root();
        let introspector = Introspector::default();
        let mut locator = Locator::default();

        let mut engine = Engine {
            world: world.track(),
            introspector: introspector.track(),
            route,
            locator: &mut locator,
            tracer: tracer.track_mut(),
        };

        let root = parse(CODE);

        // Check for well-formedness unless we are in trace mode.
        let errors = root.errors();
        if !errors.is_empty() {
            panic!(
                "{:#?}",
                errors.into_iter().map(Into::into).collect::<Vec<SourceDiagnostic>>()
            );
        }

        let markup = root.cast::<ast::Markup>().unwrap();

        let global = Library::builder().build();
        let module = markup.compile_all(&mut engine, "top", Some(global)).unwrap();

        let module = module.eval(&mut engine).unwrap();
        panic!("{:#?}", module.content());
    }

    impl BenchWorld {
        fn new() -> Self {
            let font = Font::new(FONT.into(), 0).unwrap();
            let book = FontBook::from_fonts([&font]);

            Self {
                library: Prehashed::new(Library::default()),
                book: Prehashed::new(book),
                font,
                source: Source::detached(CODE),
            }
        }

        fn track(&self) -> Tracked<dyn World> {
            (self as &dyn World).track()
        }
    }

    impl World for BenchWorld {
        fn library(&self) -> &Prehashed<Library> {
            &self.library
        }

        fn book(&self) -> &Prehashed<FontBook> {
            &self.book
        }

        fn main(&self) -> Source {
            self.source.clone()
        }

        fn source(&self, _: FileId) -> FileResult<Source> {
            unimplemented!()
        }

        fn file(&self, _: FileId) -> FileResult<Bytes> {
            unimplemented!()
        }

        fn font(&self, _: usize) -> Option<Font> {
            Some(self.font.clone())
        }

        fn today(&self, _: Option<i64>) -> Option<Datetime> {
            unimplemented!()
        }
    }
}
