use typst_syntax::Span;

use crate::foundations::{Label, Value};
use crate::util::PicoStr;
use crate::vm::{Access, Pattern, Readable, Register};
use crate::Library;

use super::{CompiledClosure, Opcode};

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
    pub accesses: Box<[Access]>,
    /// The list of labels.
    pub labels: Box<[Label]>,
    /// The list of patterns.
    pub patterns: Box<[Pattern]>,
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
