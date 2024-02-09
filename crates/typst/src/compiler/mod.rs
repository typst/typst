mod access;
mod binding;
mod call;
mod closure;
mod code;
mod compiled;
mod flow;
mod include;
mod instructions;
mod markup;
mod math;
mod module;
mod ops;
mod pattern;
mod register;
mod remapper;
mod rules;
mod scope;

use std::cell::RefCell;
use std::rc::Rc;

use ecow::EcoString;
use typst_syntax::Span;

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{IntoValue, Label, Str, Value};
use crate::util::PicoStr;
use crate::vm::{
    Access as VmAccess, AccessId, ClosureId, Constant, LabelId, Pattern as VmPattern,
    PatternId, Pointer, SpanId, StringId, Writable,
};
use crate::Library;

pub use self::access::*;
pub use self::closure::*;
pub use self::compiled::*;
pub use self::instructions::*;
pub use self::module::*;
pub use self::pattern::*;
pub use self::register::*;
pub use self::remapper::*;
pub use self::scope::*;

const DEFAULT_CAPACITY: usize = 8 << 10;

pub struct Compiler {
    /// The raw instruction buffer.
    instructions: Vec<Opcode>,
    /// The span buffer.
    spans: Vec<Span>,
    /// The current scope.
    pub scope: Rc<RefCell<CompilerScope>>,
    /// The function name (if any).
    pub name: Option<PicoStr>,
    /// The common values between scopes.
    common: Inner,
}

impl Compiler {
    /// Creates a new compiler for a module.
    pub fn module(library: Library) -> Self {
        Self {
            instructions: Vec::with_capacity(DEFAULT_CAPACITY),
            spans: Vec::with_capacity(DEFAULT_CAPACITY),
            scope: Rc::new(RefCell::new(CompilerScope::module(library))),
            name: None,
            common: Inner::new(),
        }
    }

    /// Creates a new compiler for a function.
    pub fn function(parent: &Self, name: impl Into<PicoStr>) -> Self {
        let parent = parent.scope.clone();
        Self {
            instructions: Vec::with_capacity(DEFAULT_CAPACITY),
            spans: Vec::with_capacity(DEFAULT_CAPACITY),
            scope: Rc::new(RefCell::new(CompilerScope::function(parent))),
            name: Some(name.into()),
            common: Inner::new(),
        }
    }

    /// Get the global library.
    pub fn library(&self) -> Library {
        self.scope.borrow().global().clone()
    }

    /// Whether we are in a function.
    pub fn in_function(&self) -> bool {
        self.scope.borrow().in_function()
    }

    /// Whether we are in a loop.
    pub fn in_loop(&self) -> bool {
        self.scope.borrow().in_loop()
    }

    /// Allocates a new register.
    pub fn register(&self) -> RegisterGuard {
        self.scope.borrow().register()
    }

    /// Allocates a pristine register.
    pub fn pristine_register(&self) -> RegisterGuard {
        self.scope.borrow().pristine_register()
    }

    /// Declares a new variable.
    pub fn declare(&self, span: Span, name: impl Into<PicoStr>) -> RegisterGuard {
        self.scope.borrow_mut().declare(span, name.into())
    }

    /// Declares a new variable.
    pub fn declare_into(
        &self,
        span: Span,
        name: impl Into<PicoStr>,
        output: impl Into<RegisterGuard>,
    ) {
        self.scope.borrow_mut().declare_into(span, name.into(), output.into())
    }

    /// Declares a new variable.
    pub fn declare_default(
        &self,
        span: Span,
        name: impl Into<PicoStr>,
        default: impl IntoValue,
    ) -> RegisterGuard {
        self.scope.borrow_mut().declare_with_default(
            span,
            name.into(),
            default.into_value(),
        )
    }

    /// Read the default value of a variable.
    pub fn default(&mut self, name: &str) -> StrResult<Option<Value>> {
        self.scope.borrow().default(name)
    }

    /// Read a variable.
    pub fn read(
        &mut self,
        span: Span,
        name: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        self.scope.borrow_mut().read(span, name)
    }

    /// Read a math variable.
    pub fn read_math(
        &mut self,
        span: Span,
        name: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        self.scope.borrow_mut().read_math(span, name)
    }

    /// Enter a new scope.
    pub fn enter(
        &mut self,
        engine: &mut Engine,
        span: Span,
        looping: bool,
        output: Writable,
        f: impl FnOnce(&mut Self, &mut Engine) -> SourceResult<bool>,
    ) -> SourceResult<()> {
        self.enter_indefinite(engine, looping, f, |compiler, _, len, is_content| {
            compiler.enter_isr(span, len as u32, is_content, output);

            Ok(())
        })
    }

    pub fn enter_indefinite<O>(
        &mut self,
        engine: &mut Engine,
        looping: bool,
        f: impl FnOnce(&mut Self, &mut Engine) -> SourceResult<O>,
        pre: impl FnOnce(&mut Self, &mut Engine, usize, O) -> SourceResult<()>,
    ) -> SourceResult<()> {
        let mut scope =
            Rc::new(RefCell::new(CompilerScope::scope(self.scope.clone(), looping)));
        let mut instructions = Vec::with_capacity(DEFAULT_CAPACITY * 8);
        let mut spans = Vec::with_capacity(DEFAULT_CAPACITY);

        self.common.scopes += 1;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.spans, &mut spans);

        let out = f(self, engine)?;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.spans, &mut spans);

        let len = instructions.len();
        pre(self, engine, len, out)?;

        self.spans.extend(spans);
        self.instructions.extend(instructions);

        Ok(())
    }

    /// Allocates a new constant.
    pub fn const_(&mut self, value: impl IntoValue) -> Constant {
        self.common.constants.insert(value.into_value())
    }

    /// Allocates a new string.
    pub fn string(&mut self, value: impl Into<EcoString>) -> StringId {
        self.common.strings.insert(Value::Str(Str::from(value.into())))
    }

    /// Allocates a new label.
    pub fn label(&mut self, label: &str) -> LabelId {
        self.common.labels.insert(Label::new(label))
    }

    /// Allocates a new closure.
    pub fn closure(&mut self, value: CompiledClosure) -> ClosureId {
        self.common.closures.insert(value)
    }

    /// Allocates a new access.
    pub fn access(&mut self, value: VmAccess) -> AccessId {
        self.common.accesses.insert(value)
    }

    /// Allocates a new pattern.
    pub fn pattern(&mut self, value: VmPattern) -> PatternId {
        self.common.patterns.insert(value)
    }

    pub fn span(&mut self, span: Span) -> SpanId {
        self.common.spans.insert(span)
    }

    pub fn remapped_instructions(&self) -> Vec<usize> {
        let mut iter = self.instructions.iter();
        let mut jumps = vec![usize::MAX; self.common.jumps as usize];

        fn remap<'a>(
            iter: &mut dyn Iterator<Item = &'a Opcode>,
            jumps: &mut Vec<usize>,
            count: &mut usize,
        ) {
            let mut i = 0;
            while let Some(next) = iter.next() {
                match next {
                    Opcode::PointerMarker(id) => {
                        jumps[id.marker.as_raw() as usize] = i;
                        *count -= 1;
                    }
                    Opcode::Enter(enter) => {
                        remap(&mut iter.take(enter.len as usize), jumps, count);
                        i += enter.len as usize;
                    }
                    Opcode::While(while_) => {
                        remap(&mut iter.take(while_.len as usize), jumps, count);
                        i += while_.len as usize;
                    }
                    Opcode::Iter(iter_op) => {
                        remap(&mut iter.take(iter_op.len as usize), jumps, count);
                        i += iter_op.len as usize;
                    }
                    _ => {}
                }

                i += 1;
                if *count == 0 {
                    break;
                }
            }
        }

        if self.common.jumps > 0 {
            let mut i = self.common.jumps as usize;
            remap(&mut iter, &mut jumps, &mut i);
        }

        if jumps.iter().any(|i| *i == usize::MAX) {
            unreachable!("unresolved jumps: {:?}", jumps);
        }

        jumps
    }

    pub fn finish_closure(
        mut self,
        span: Span,
        params: Vec<CompiledParam>,
        self_storage: Option<RegisterGuard>,
    ) -> CompiledCode {
        let scopes = self.scope.borrow();
        let captures: Vec<_> = scopes
            .captures
            .values()
            .map(|capture| CodeCapture {
                name: capture.name.clone(),
                span: capture.span,
                readable: capture.readable.as_readable(),
                register: capture.register.as_register(),
            })
            .collect();

        let jumps = self.remapped_instructions();
        self.instructions.shrink_to_fit();
        self.spans.shrink_to_fit();
        let registers = scopes.registers.borrow().len() as usize;
        CompiledCode {
            defaults: self.get_default_scope().into(),
            span,
            registers,
            name: self.name,
            instructions: self.instructions.into(),
            spans: self.spans.into(),
            global: scopes.global().clone(),
            constants: self.common.constants.into_values().into(),
            strings: self.common.strings.into_values().into(),
            closures: self.common.closures.into_values().into(),
            accesses: self.common.accesses.into_values().into(),
            labels: self.common.labels.into_values().into(),
            patterns: self.common.patterns.into_values().into(),
            isr_spans: self.common.spans.into_values().into(),
            jumps: jumps.into(),
            captures: Some(captures.into()),
            params: Some(params.into()),
            self_storage: self_storage.map(|r| r.as_register()),
            exports: None,
        }
    }

    pub fn finish_module(
        mut self,
        span: Span,
        name: impl Into<PicoStr>,
        mut exports: Vec<Export>,
    ) -> CompiledCode {
        let scopes = self.scope.borrow();
        debug_assert!(scopes.captures.is_empty());

        let jumps = self.remapped_instructions();
        self.instructions.shrink_to_fit();
        self.spans.shrink_to_fit();
        exports.shrink_to_fit();
        let registers = scopes.registers.borrow().len() as usize;
        CompiledCode {
            defaults: self.get_default_scope().into(),
            span,
            registers,
            name: Some(name.into()),
            instructions: self.instructions.into(),
            spans: self.spans.into(),
            global: scopes.global().clone(),
            constants: self.common.constants.into_values().into(),
            strings: self.common.strings.into_values().into(),
            closures: self.common.closures.into_values().into(),
            accesses: self.common.accesses.into_values().into(),
            labels: self.common.labels.into_values().into(),
            patterns: self.common.patterns.into_values().into(),
            isr_spans: self.common.spans.into_values().into(),
            jumps: jumps.into(),
            captures: None,
            params: None,
            self_storage: None,
            exports: Some(exports.into()),
        }
    }

    pub fn get_default_scope(&self) -> Vec<DefaultValue> {
        self.scope
            .borrow()
            .variables
            .values()
            .filter_map(|v| v.default.clone().map(|d| (d, v.register.as_register())))
            .map(|(value, target)| DefaultValue { target, value })
            .collect::<Vec<_>>()
    }

    pub fn flow(&mut self) {
        self.instructions.push(Opcode::Flow);
        self.spans.push(Span::detached());
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn spans(&self) -> usize {
        self.spans.len()
    }

    pub fn marker(&mut self) -> Pointer {
        let id = self.common.jumps;
        self.common.jumps += 1;
        Pointer::new(id)
    }
}

#[derive(Default)]
struct Inner {
    /// The constant remapper.
    constants: Remapper<Constant, Value>,
    /// The string remapper.
    strings: Remapper<StringId, Value>,
    /// The label remapper.
    labels: Remapper<LabelId, Label>,
    /// The closur remapper.
    closures: Remapper<ClosureId, CompiledClosure>,
    /// The access remapper.
    accesses: Remapper<AccessId, VmAccess>,
    /// The pattern remapper.
    patterns: Remapper<PatternId, VmPattern>,
    /// The span remapper.
    spans: Remapper<SpanId, Span>,
    /// The current scope counter.
    scopes: u16,
    /// The current jump counter.
    jumps: u16,
}

impl Inner {
    /// Creates a new inner.
    fn new() -> Self {
        Self::default()
    }
}

pub trait CompileTopLevel {
    fn compile_top_level(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<()>;
}

pub trait Compile {
    type Output;

    type IntoOutput;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()>;

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput>;
}
