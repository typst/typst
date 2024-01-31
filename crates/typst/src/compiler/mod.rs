mod access;
mod binding;
mod call;
mod closure;
mod code;
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
use std::sync::Arc;

use ecow::{EcoString, EcoVec};
use typst_syntax::Span;

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{IntoValue, Label, Str, Value};
use crate::vm::{
    self, Access as VmAccess, AccessId, ClosureId, CompiledClosure, CompiledParam,
    Constant, DefaultValue, LabelId, OptionalWritable, Pattern as VmPattern, PatternId,
    ScopeId, StringId, Writable,
};
use crate::Library;

pub use self::access::*;
pub use self::instructions::*;
pub use self::markup::*;
pub use self::module::*;
pub use self::pattern::*;
pub use self::register::*;
pub use self::remapper::*;
pub use self::scope::*;

const DEFAULT_CAPACITY: usize = 8 << 10;

pub struct Compiler {
    /// The raw instruction buffer.
    pub instructions: Vec<Opcode>,
    /// The current scope.
    pub scope: Rc<RefCell<CompilerScope>>,
    /// The function name (if any).
    pub name: Option<EcoString>,
    /// The common values between scopes.
    common: Inner,
    /// The current scope ID.
    scope_id: Option<ScopeId>,
}

impl Compiler {
    /// Creates a new compiler for a module.
    pub fn module(name: &str, library: Library) -> Self {
        Self {
            instructions: Vec::with_capacity(DEFAULT_CAPACITY),
            scope: Rc::new(RefCell::new(CompilerScope::module(library))),
            name: Some(name.into()),
            common: Inner::new(),
            scope_id: None,
        }
    }

    /// Creates a new compiler for a function.
    pub fn function(parent: &Self, name: impl Into<EcoString>) -> Self {
        let parent = parent.scope.clone();
        Self {
            instructions: Vec::with_capacity(DEFAULT_CAPACITY),
            scope: Rc::new(RefCell::new(CompilerScope::function(parent))),
            name: Some(name.into()),
            common: Inner::new(),
            scope_id: None,
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
    pub fn register(&self) -> StrResult<RegisterGuard> {
        self.scope.borrow().register()
    }

    /// Allocates a pristine register.
    pub fn pristine_register(&self) -> StrResult<RegisterGuard> {
        self.scope.borrow().pristine_register()
    }

    /// Declares a new variable.
    pub fn declare(
        &self,
        span: Span,
        name: impl Into<EcoString>,
    ) -> StrResult<RegisterGuard> {
        self.scope.borrow_mut().declare(span, name.into())
    }

    /// Declares a new variable.
    pub fn declare_default(
        &self,
        span: Span,
        name: impl Into<EcoString>,
        default: impl IntoValue,
    ) -> StrResult<RegisterGuard> {
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
        span: Span,
        looping: bool,
        joining: Option<Writable>,
        mut display: bool,
        f: impl FnOnce(&mut Self, &mut bool) -> SourceResult<()>,
    ) -> SourceResult<()> {
        let mut scope_id = Some(ScopeId::new(self.common.scopes));
        let mut scope =
            Rc::new(RefCell::new(CompilerScope::scope(self.scope.clone(), looping)));
        let mut instructions = Vec::with_capacity(DEFAULT_CAPACITY);

        self.common.scopes += 1;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.scope_id, &mut scope_id);

        f(self, &mut display)?;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.scope_id, &mut scope_id);

        let out = match joining {
            Some(out) => OptionalWritable::some(out),
            None => OptionalWritable::none(),
        };

        let defaults = scope
            .borrow()
            .variables
            .values()
            .filter_map(|v| v.default.clone().map(|d| (d, v.register.as_register())))
            .map(|(value, target)| DefaultValue { target, value })
            .collect::<EcoVec<_>>();

        self.common.defaults.push(defaults);
        let scope_id = scope_id.unwrap();

        let len = instructions.iter().map(Write::size).sum::<usize>();
        self.instructions.push(Opcode::enter(
            span,
            len as u32,
            scope_id,
            0b000
                | if looping { 0b001 } else { 0b000 }
                | if joining.is_some() { 0b010 } else { 0b000 }
                | if display { 0b100 } else { 0b000 },
            out,
        ));

        self.instructions.extend(instructions);

        Ok(())
    }

    pub fn enter_indefinite(
        &mut self,
        engine: &mut Engine,
        looping: bool,
        joining: Option<Writable>,
        mut display: bool,
        f: impl FnOnce(&mut Self, &mut Engine, &mut bool) -> SourceResult<()>,
        pre: impl FnOnce(
            &mut Self,
            &mut Engine,
            usize,
            OptionalWritable,
            ScopeId,
        ) -> SourceResult<()>,
    ) -> SourceResult<()> {
        let mut scope_id = Some(ScopeId::new(self.common.scopes));
        let mut scope =
            Rc::new(RefCell::new(CompilerScope::scope(self.scope.clone(), looping)));
        let mut instructions = Vec::with_capacity(DEFAULT_CAPACITY);

        self.common.scopes += 1;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.scope_id, &mut scope_id);

        f(self, engine, &mut display)?;

        std::mem::swap(&mut self.scope, &mut scope);
        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.scope_id, &mut scope_id);

        let out = match joining {
            Some(out) => OptionalWritable::some(out),
            None => OptionalWritable::none(),
        };

        let defaults = scope
            .borrow()
            .variables
            .values()
            .filter_map(|v| v.default.clone().map(|d| (d, v.register.as_register())))
            .map(|(value, target)| DefaultValue { target, value })
            .collect::<EcoVec<_>>();

        self.common.defaults.push(defaults);
        let scope_id = scope_id.unwrap();

        let len = instructions.iter().map(Write::size).sum::<usize>();
        pre(self, engine, len, out, scope_id)?;

        self.instructions.extend(instructions);

        Ok(())
    }

    /// Get the current scope ID.
    pub fn scope_id(&self) -> Option<ScopeId> {
        self.scope_id
    }

    /// Push a new instruction.
    pub fn isr(&mut self, isr: impl Into<Opcode>) {
        self.instructions.push(isr.into());
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

    /// Allocates a new jump label.
    pub fn jump(&mut self) -> JumpLabel {
        let jump = self.common.jump;
        self.common.jump += 1;
        JumpLabel(jump)
    }

    pub fn into_compiled_closure(
        mut self,
        span: Span,
        params: Vec<CompiledParam>,
        self_storage: Option<WritableGuard>,
    ) -> CompiledClosure {
        let scopes = self.scope.borrow();
        let captures = scopes
            .captures
            .values()
            .map(|capture| vm::Capture {
                name: capture.name.clone(),
                value: capture.readable.as_readable(),
                location: capture.register.as_writeable(),
                span: capture.span,
            })
            .collect();

        let mut instructions = Vec::with_capacity(1 << 20);
        self.instructions
            .iter()
            .for_each(|isr| isr.write(&self.instructions, &mut instructions));
        instructions.shrink_to_fit();

        self.common.defaults.insert(0, self.get_default_scope());

        CompiledClosure {
            inner: Arc::new(vm::Inner {
                name: self.name,
                span,
                instructions,
                global: scopes.global().clone(),
                constants: self.common.constants.into_values(),
                strings: self.common.strings.into_values(),
                closures: self.common.closures.into_values(),
                accesses: self.common.accesses.into_values(),
                labels: self.common.labels.into_values(),
                patterns: self.common.patterns.into_values(),
                defaults: self.common.defaults,
                output: None,
                joined: true,
            }),
            captures,
            params,
            self_storage: self_storage.map(|r| r.as_writable()),
        }
    }

    pub fn get_default_scope(&self) -> EcoVec<DefaultValue> {
        self.scope
            .borrow()
            .variables
            .values()
            .filter_map(|v| v.default.clone().map(|d| (d, v.register.as_register())))
            .map(|(value, target)| DefaultValue { target, value })
            .collect::<EcoVec<_>>()
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
    /// The current scope counter.
    scopes: u16,
    /// The default value remapper.
    defaults: Vec<EcoVec<DefaultValue>>,
    /// The jump label counter.
    jump: u16,
}

impl Inner {
    /// Creates a new inner.
    fn new() -> Self {
        Self::default()
    }
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
