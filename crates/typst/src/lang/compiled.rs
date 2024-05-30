use std::sync::Arc;

use smallvec::SmallVec;
use typst_syntax::Span;
use typst_utils::LazyHash;

use crate::foundations::{Label, Module, Value};
use crate::utils::PicoStr;
use crate::Library;

use super::closure::Closure;
use super::compiler::Compiler;
use super::opcodes::{AccessId, Opcode, PatternId, Readable, Writable};
use super::operands::{Register, StringId};

pub enum CompiledModule {
    /// A module that needs to be resolved dynamically but for which we
    /// know the exports the user is interested in.
    Dynamic(Arc<LazyHash<CompiledCode>>),
    /// A module that has been instantiated statically.
    Instanciated(Module),
}

impl CompiledModule {
    /// Checks whether the module is dynamic.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }

    /// Returns the name of the module.
    pub fn name(&self) -> &str {
        "unknown"
    }
}

#[derive(Clone, Hash)]
pub struct CompiledCode {
    /// The name of the code.
    pub name: Option<PicoStr>,
    /// The span where the code was defined.
    pub span: Span,
    /// The instructions as byte code.
    pub instructions: Box<[Opcode]>,
    /// The spans of the instructions.
    pub spans: Box<[Span]>,
    /// The global library.
    pub global: Library,
    /// The number of registers needed for the code.
    pub registers: usize,
    /// The list of constants.
    pub constants: Box<[Value]>,
    /// The list of strings.
    pub strings: Box<[Value]>,
    /// The list of closures.
    pub closures: Box<[CompiledClosure]>,
    /// The accesses.
    pub accesses: Box<[CompiledAccess]>,
    /// The list of labels.
    pub labels: Box<[Label]>,
    /// The list of patterns.
    pub patterns: Box<[CompiledPattern]>,
    /// The default values of variables.
    pub defaults: Box<[DefaultValue]>,
    /// The spans used in the code.
    pub isr_spans: Box<[Span]>,
    /// The jumps used in the code.
    pub jumps: Box<[usize]>,
    /// The exports of the module (empty for closures).
    pub exports: Option<Box<[Export]>>,
    /// The captures of the code (empty for modules).
    pub captures: Option<Box<[CodeCapture]>>,
    /// The parameters of the code(empty for modules).
    pub params: Option<Box<[CompiledParam]>>,
    /// Where to store the reference to the closure itself(empty for modules).
    pub self_storage: Option<Register>,
}

/// A closure that has been compiled but is not yet instantiated.
#[derive(Clone, Hash, PartialEq)]
pub enum CompiledClosure {
    /// A closure that has been compiled but is not yet instantiated.
    Closure(Arc<LazyHash<CompiledCode>>),
    /// A closure that has been instantiated statically.
    ///
    /// This is used for closures that do not capture any variables.
    /// The closure is already compiled and can be used directly.
    Instanciated(Closure),
}

impl CompiledClosure {
    pub fn new(resource: CompiledCode, compiler: &Compiler) -> Self {
        // Check whether we have any defaults that are resolved at runtime.
        let has_defaults = resource
            .params
            .iter()
            .flat_map(|param| param.iter())
            .filter_map(|param| param.default())
            .any(|default| default.is_reg());

        // Check if we have any captures.
        let has_captures = !resource.captures.as_ref().map_or(false, |c| c.is_empty());

        if has_defaults || has_captures {
            Self::Closure(Arc::new(LazyHash::new(resource)))
        } else {
            Self::Instanciated(Closure::no_instance(resource, compiler))
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct CompiledPattern {
    pub span: Span,
    pub kind: CompiledPatternKind,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum CompiledPatternKind {
    /// Destructure into a single local.
    Single(CompiledPatternItem),

    /// Destructure into a tuple of locals.
    Tuple(SmallVec<[CompiledPatternItem; 2]>, bool),
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum CompiledPatternItem {
    /// Destructure into a single local.
    Placeholder(Span),

    /// Destructure into a single local.
    Simple(Span, AccessId, StringId),

    /// Destructure into a nested pattern.
    Nested(Span, PatternId),

    /// Spread the remaining values into a single value.
    Spread(Span, AccessId),

    /// Spread the remaining values into a single value and discard it.
    SpreadDiscard(Span),

    /// A named pattern.
    Named(Span, AccessId, StringId),
}

#[derive(Debug, Clone, Hash)]
pub struct Export {
    /// The name of the export.
    pub name: PicoStr,
    /// The value of the export.
    pub value: Readable,
    /// The span where the export was defined.
    pub span: Span,
}

#[derive(Clone, Hash)]
pub struct DefaultValue {
    /// The value of the default.
    pub value: Value,
    /// The target where the default value will be stored.
    pub target: Register,
}

#[derive(Clone, Hash, PartialEq)]
pub enum CompiledParam {
    /// A positional parameter.
    Pos(Register, PicoStr),
    /// A named parameter.
    Named {
        /// The span of the parameter.
        span: Span,
        /// The location where the parameter will be stored.
        target: Register,
        /// The name of the parameter.
        name: PicoStr,
        /// The default value of the parameter.
        default: Option<Readable>,
    },
    /// A sink parameter.
    Sink(Span, Option<Register>, PicoStr),
}

impl CompiledParam {
    pub fn default(&self) -> Option<Readable> {
        match self {
            Self::Pos(_, _) | Self::Sink(_, _, _) => None,
            Self::Named { default, .. } => *default,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct CodeCapture {
    /// The name of the value to capture.
    pub name: PicoStr,
    /// The value of the capture **in the parent scope**.
    pub readable: Readable,
    /// Where the value is stored **in the closure's scope**.
    pub register: Register,
    /// The span where the capture was occurs.
    pub span: Span,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum CompiledAccess {
    /// Access this value through a readable.
    Readable(Readable),

    /// Access this value through a writeable.
    Writable(Writable),

    /// Access this value through the global scope.
    Module(Value),

    /// Access this value through a closure.
    Func(Value),

    /// Access this value directly.
    Value(Value),

    /// Access this value through a type.
    Type(Value),

    /// Access this value through a chained access.
    ///
    /// This uses IDs in order to: avoid allocating, allow all of the accesses
    /// to be contiguous in memory.
    Chained(AccessId, PicoStr),

    /// Access this value through an accessor method.
    ///
    /// This uses IDs in order to: avoid allocating, allow all of the accesses
    /// to be contiguous in memory.
    AccessorMethod(AccessId, PicoStr, Readable),
}
