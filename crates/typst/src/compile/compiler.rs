mod binding;
mod call;
mod code;
mod flow;
mod import;
mod markup;
mod math;
mod ops;
mod rules;

use std::cell::RefCell;
use std::rc::Rc;

pub use self::binding::*;
pub use self::call::*;
pub use self::import::*;

use ecow::{eco_vec, EcoString, EcoVec};
use indexmap::IndexMap;
use typst_syntax::{
    ast::{self, AstNode},
    Span,
};

use crate::{
    diag::StrResult,
    foundations::{Scope, Value},
};
use crate::{
    diag::{bail, SourceResult},
    util::hash128,
};
use crate::{engine::Engine, foundations::Label, Library};

use super::AccessId;
use super::REGISTER_COUNT;
use super::{
    destructure::Pattern, Call, CallId, CapturedId, ClosureId, CompiledModule, ConstId,
    Instruction, IteratorId, JmpLabel, LabelId, LocalId, ModuleId, PatternId, Register,
    ScopeId, StringId,
};

#[derive(Debug, Clone, Default)]
pub struct ScopeLinkedList {
    pub top: Scope,
    pub next: Option<Box<ScopeLinkedList>>,
    pub len: usize,
}

impl ScopeLinkedList {
    pub fn enter(&mut self) {
        let next = std::mem::take(self);
        self.next = Some(Box::new(next));
    }

    pub fn exit(&mut self) {
        let mut next = self.next.take().unwrap();
        next.len += self.len;
        *self = *next;
    }

    pub fn len(&self) -> usize {
        1 + self.next.as_ref().map_or(0, |next| next.len())
    }
}

impl<'a> IntoIterator for &'a ScopeLinkedList {
    type Item = &'a Scope;
    type IntoIter = ScopeLinkedListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ScopeLinkedListIter { current: Some(self) }
    }
}

pub struct ScopeLinkedListIter<'a> {
    current: Option<&'a ScopeLinkedList>,
}

impl<'a> Iterator for ScopeLinkedListIter<'a> {
    type Item = &'a Scope;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = current.next.as_deref();
        Some(&current.top)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
#[repr(transparent)]
pub struct Registers([bool; REGISTER_COUNT as usize]);

impl Registers {
    pub fn next(&mut self) -> Option<Register> {
        self.0
            .iter_mut()
            .enumerate()
            .find_map(|(i, b)| {
                if *b {
                    None
                } else {
                    *b = true;
                    Some(i)
                }
            })
            .map(|i| Register(i as u16 + 1))
    }

    pub fn free(&mut self, register: Register) {
        self.0[register.0 as usize - 1] = false;
    }
}

pub struct Compiler<'a, 'b: 'a> {
    pub engine: &'a mut Engine<'b>,
    pub consts: IndexMap<u128, (usize, Value)>,
    pub labels: usize,
    pub spans: EcoVec<Span>,
    pub calls: EcoVec<Call>,
    pub content_labels: EcoVec<Label>,
    pub instructions: EcoVec<Instruction>,
    pub patterns: EcoVec<Pattern>,
    pub accesses: EcoVec<AccessPattern>,
    pub strings: IndexMap<u128, (usize, EcoString)>,
    pub closures: IndexMap<u128, (usize, CompiledClosure)>,
    pub scopes: CompilerScopes,
    pub iterator: u32,
    pub in_function: bool,
    pub current_name: Option<EcoString>,
    pub registers: Registers,
    pub loop_stack: Vec<(JmpLabel, JmpLabel)>,
}

pub struct CompilerScopes(Rc<RefCell<Inner>>);

pub struct Inner {
    pub base: Option<Library>,
    pub scopes: ScopeLinkedList,
    pub parent: Option<CompilerScopes>,
    pub captures: IndexMap<u128, (usize, Capture)>,
}

impl CompilerScopes {
    pub fn enter(&self) {
        self.0.borrow_mut().scopes.enter();
    }

    pub fn exit(&self) {
        self.0.borrow_mut().scopes.exit();
    }

    pub fn len(&self) -> usize {
        self.0.borrow().scopes.top.len()
    }

    pub fn set_parent(&self, parent: &CompilerScopes) {
        self.0.borrow_mut().parent = Some(CompilerScopes(Rc::clone(&parent.0)));
    }

    pub fn captures_as_vec(&self) -> EcoVec<Capture> {
        self.0.borrow().captures.values().map(|(_, c)| c).cloned().collect()
    }

    pub fn capture(&self, scope: ScopeId, local: LocalId) -> CapturedId {
        let mut this = self.0.borrow_mut();
        let captures = &mut this.captures;

        let hash = hash128(&(scope, local));
        if let Some((id, _)) = captures.get(&hash) {
            return CapturedId(*id as u16);
        }

        let id = captures.len();
        captures.insert(hash, (id, Capture::Local { scope, local }));
        CapturedId(id as u16)
    }

    pub fn capture_captured(&self, captured: CapturedId) -> CapturedId {
        let mut this = self.0.borrow_mut();
        let captures = &mut this.captures;

        let hash = hash128(&captured);
        if let Some((id, _)) = captures.get(&hash) {
            return CapturedId(*id as u16);
        }

        let id = captures.len();
        captures.insert(hash, (id, Capture::Captured { captured }));
        CapturedId(id as u16)
    }

    pub fn local(&self, name: EcoString) -> LocalId {
        self.local_with_default(name, Value::None)
    }

    pub fn local_with_default(&self, name: EcoString, default: Value) -> LocalId {
        let mut this = self.0.borrow_mut();
        let idx = this.scopes.top.define(name, default);
        LocalId(idx as u16)
    }

    pub fn local_ref(&self, name: &EcoString, target: Register) -> Option<Instruction> {
        fn inner_local_ref(
            scopes: &CompilerScopes,
            name: &EcoString,
            target: Register,
        ) -> Option<Instruction> {
            let inner = scopes.0.borrow();
            if let Some((scope, local)) = inner
                .scopes
                .into_iter()
                .enumerate()
                .find_map(|(i, scope)| scope.get_index(name).map(|index| (i, index)))
            {
                Some(Instruction::Load {
                    scope: ScopeId(scope as u16),
                    local: LocalId(local as u16),
                    target,
                })
            } else if let Some(idx) = inner
                .base
                .as_ref()
                .and_then(|base| base.global.field_index(name).ok())
            {
                Some(Instruction::LoadModule {
                    module: ModuleId::Global,
                    local: LocalId(idx as u16),
                    target,
                })
            } else if let Some(parent) = &inner.parent {
                let local = inner_local_ref(parent, name, target)?;
                drop(inner);
                match local {
                    Instruction::Load { scope, local, .. } => {
                        let capture = scopes.capture(scope, local);
                        Some(Instruction::LoadCaptured { capture, target })
                    }
                    Instruction::LoadCaptured { capture, .. } => {
                        let capture = scopes.capture_captured(capture);
                        Some(Instruction::LoadCaptured { capture, target })
                    }
                    _ => unreachable!(),
                }
            } else {
                None
            }
        }

        inner_local_ref(&self, name, target)
    }

    pub fn local_ref_in_math(
        &self,
        name: &EcoString,
        target: Register,
    ) -> Option<Instruction> {
        if let Some(isr) = self.local_ref(name, target) {
            Some(isr)
        } else if let Some(idx) = self
            .0
            .borrow()
            .base
            .as_ref()
            .and_then(|base| base.math.field_index(name).ok())
        {
            Some(Instruction::LoadModule {
                module: ModuleId::Math,
                local: LocalId(idx as u16),
                target,
            })
        } else {
            None
        }
    }
}

impl<'a, 'b> Compiler<'a, 'b> {
    pub fn new(
        engine: &'a mut Engine<'b>,
        parents: Option<ScopeLinkedList>,
        base: Option<Library>,
        in_function: bool,
    ) -> Self {
        Self {
            engine,
            consts: IndexMap::new(),
            spans: EcoVec::new(),
            calls: EcoVec::new(),
            content_labels: EcoVec::new(),
            strings: IndexMap::new(),
            labels: 0,
            iterator: 0,
            instructions: EcoVec::new(),
            patterns: EcoVec::new(),
            accesses: EcoVec::new(),
            scopes: CompilerScopes(Rc::new(RefCell::new(Inner {
                base,
                scopes: ScopeLinkedList {
                    top: Scope::new(),
                    len: parents.as_ref().map(|p| p.len).unwrap_or_default(),
                    next: parents.map(Box::new),
                },
                parent: None,
                captures: IndexMap::new(),
            }))),
            closures: IndexMap::new(),
            loop_stack: Vec::new(),
            in_function,
            current_name: None,
            registers: Registers::default(),
        }
    }

    pub fn in_scope<T>(
        &mut self,
        span: Span,
        f: impl FnOnce(&mut Compiler) -> SourceResult<T>,
    ) -> SourceResult<T> {
        self.scopes.enter();

        let mut instructions = std::mem::take(&mut self.instructions);
        let mut spans = std::mem::take(&mut self.spans);

        let out = f(self)?;

        std::mem::swap(&mut self.instructions, &mut instructions);
        std::mem::swap(&mut self.spans, &mut spans);

        let len = self.scopes.len();
        self.spans.push(span);
        self.instructions.push(Instruction::Enter { size: len });

        self.instructions.extend(instructions.into_iter());
        self.spans.extend(spans.into_iter());

        self.spans.push(span);
        self.instructions.push(Instruction::Exit {});

        self.scopes.exit();

        Ok(out)
    }

    pub fn with_parent<'c, 'd>(self, parent: &CompilerScopes) -> Compiler<'a, 'b> {
        self.scopes.set_parent(parent);
        self
    }

    pub fn consts_as_vec(&self) -> EcoVec<Value> {
        self.consts
            .values()
            .map(|(_, value)| value)
            .cloned()
            .collect::<EcoVec<_>>()
    }

    pub fn strings_as_vec(&self) -> EcoVec<EcoString> {
        self.strings.values().map(|(_, s)| s).cloned().collect()
    }

    pub fn patterns_as_vec(&self) -> EcoVec<Pattern> {
        self.patterns.clone()
    }

    pub fn closures_as_vec(&self) -> EcoVec<CompiledClosure> {
        self.closures.values().map(|(_, c)| c).cloned().collect()
    }

    pub fn captures_as_vec(&self) -> EcoVec<Capture> {
        self.scopes.captures_as_vec()
    }

    pub fn capture(&self, scope: ScopeId, local: LocalId) -> CapturedId {
        self.scopes.capture(scope, local)
    }

    pub fn capture_captured(&self, captured: CapturedId) -> CapturedId {
        self.scopes.capture_captured(captured)
    }

    pub fn closure(&mut self, closure: CompiledClosure) -> ClosureId {
        let hash = hash128(&closure);
        if let Some((id, _)) = self.closures.get(&hash) {
            return ClosureId(*id as u16);
        }

        let id = self.closures.len();
        self.closures.insert(hash, (id, closure));
        ClosureId(id as u16)
    }

    pub fn string(&mut self, value: &EcoString) -> StringId {
        let hash = hash128(&value);
        if let Some((id, _)) = self.strings.get(&hash) {
            return StringId(*id as u16);
        }

        let id = self.strings.len() as u16;
        self.strings.insert(hash, (id as usize, value.clone()));
        StringId(id)
    }

    /// Returns the labels in the order they were defined.
    pub fn labels_as_vec(&self) -> EcoVec<usize> {
        let mut labels = eco_vec![0; self.labels];
        for (i, instruction) in self.instructions.iter().enumerate() {
            if let Instruction::Label { label } = instruction {
                labels.make_mut()[label.0 as usize] = i;
            }
        }
        labels
    }

    pub fn content_label(&mut self, label: &str) -> LabelId {
        if let Some(index) = self.content_labels.iter().position(|l| l.as_str() == label)
        {
            return LabelId(index as u16);
        }

        let index = self.content_labels.len() as u16;
        self.content_labels.push(Label::new(label));
        LabelId(index)
    }

    pub fn label(&mut self) -> JmpLabel {
        let label = self.labels;
        self.labels += 1;
        JmpLabel(label as u16)
    }

    pub fn pattern(&mut self, pattern: Pattern) -> PatternId {
        self.patterns.push(pattern);
        PatternId(self.patterns.len() as u16 - 1)
    }

    pub fn access(&mut self, access: AccessPattern) -> AccessId {
        self.accesses.push(access);
        AccessId(self.accesses.len() as u16 - 1)
    }

    pub fn const_(&mut self, value: Value) -> ConstId {
        let hash = hash128(&value);
        if let Some((id, _)) = self.consts.get(&hash) {
            return ConstId(*id as u16);
        }

        let id = self.consts.len() as u16;
        self.consts.insert(hash, (id as usize, value));
        ConstId(id)
    }

    pub fn local_ref_in_math(
        &mut self,
        name: &EcoString,
        target: Register,
    ) -> Option<Instruction> {
        self.scopes.local_ref_in_math(name, target)
    }

    pub fn local_ref(&self, name: &EcoString, target: Register) -> Option<Instruction> {
        self.scopes.local_ref(name, target)
    }

    pub fn local(&self, _: Span, name: EcoString) -> LocalId {
        self.scopes.local(name)
    }

    pub fn iterator(&mut self) -> IteratorId {
        let index = self.iterator as u16;
        self.iterator += 1;
        IteratorId(index)
    }

    pub fn pop_iterator(&mut self) {
        self.iterator -= 1;
    }

    pub fn reg(&mut self) -> StrResult<Register> {
        let Some(reg) = self.registers.next() else {
            bail!("ran out of registers while compiling code.");
        };

        Ok(reg)
    }

    pub fn use_reg(&mut self, reg: Register) -> StrResult<()> {
        if reg.is_none() {
            return Ok(());
        }

        if self.registers.0[reg.0 as usize - 1] {
            bail!("register {} is already in use", reg.0)
        }

        self.registers.0[reg.0 as usize - 1] = true;
        Ok(())
    }

    pub fn free(&mut self, reg: Register) {
        if reg.is_none() {
            return;
        }

        self.registers.free(reg);
    }

    pub fn call(&mut self, call: Call) -> CallId {
        let index = self.calls.len() as u16;
        self.calls.push(call);
        CallId(index)
    }
}

pub trait Compile {
    #[typst_macros::time(name = "bytecode compilation")]
    fn compile_all<'a, 'b, 'c: 'a>(
        &self,
        engine: &'a mut Engine<'c>,
        name: impl Into<EcoString>,
        base: Option<Library>,
    ) -> SourceResult<CompiledModule> {
        // Create a new scope for the module.
        let mut compiler = Compiler::new(engine, None, base, false);

        // Compile the module.
        let output = self.compile(&mut compiler)?;

        // Then we can create the compiled module.
        let scopes = compiler.scopes.0.borrow();
        Ok(CompiledModule {
            name: name.into(),
            output,
            closures: compiler.closures_as_vec(),
            constants: compiler.consts_as_vec(),
            strings: compiler.strings_as_vec(),
            patterns: compiler.patterns_as_vec(),
            labels: compiler.labels_as_vec(),
            locals: scopes.scopes.top.len(),
            instructions: compiler.instructions,
            spans: compiler.spans,
            content_labels: compiler.content_labels,
            calls: compiler.calls,
            scope: scopes.scopes.top.clone(),
            accesses: compiler.accesses,
        })
    }

    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register>;

    fn compile_display(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = self.compile(compiler)?;
        compiler
            .spans
            .push(compiler.spans.last().copied().unwrap_or_else(Span::detached));
        compiler
            .instructions
            .push(Instruction::Display { value, target: value });

        Ok(value)
    }
}

impl Compile for ast::FuncReturn<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        if !compiler.in_function {
            bail!(self.span(), "cannot return outside of function");
        }

        let body = self
            .body()
            .map_or(Ok(Register::NONE), |value| value.compile(compiler))?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Return { register: body });

        compiler.free(body);

        Ok(Register::NONE)
    }
}
