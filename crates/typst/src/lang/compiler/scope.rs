use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use typst_syntax::Span;
use typst_utils::PicoStr;

use crate::diag::{bail, StrResult};
use crate::foundations::Value;
use crate::lang::operands::{Global, Math};
use crate::Library;

use super::{PristineRegisterGuard, ReadableGuard, RegisterAllocator, RegisterGuard};

bitflags::bitflags! {
    pub struct ScopeFlags: u8 {
        const NONE = 0b0000;
        const IN_FUNCTION = 0b0001;
        const IN_LOOP = 0b0010;
    }
}

pub struct Scope<'lib> {
    /// The current scope flags.
    pub flags: ScopeFlags,

    /// The global library.
    /// This is the scope from which values can be read, but not written, and must not be captured to be used.
    /// If this is `None`, must use the parent scope's register allocator.
    pub global: Option<&'lib Library>,

    /// The parent scope.
    /// This is the scope from which valeus can be both read and written, without needing to capture them.
    pub parent: Option<Rc<RefCell<Scope<'lib>>>>,

    /// The capturing scope.
    /// This is the scope from which values can be read, but not written, and must be captured to be used.
    pub capturing: Option<Rc<RefCell<Scope<'lib>>>>,

    /// The register allocator for this scope, if any.
    /// If this is `None`, must use the parent scope's register allocator.
    pub registers: Option<RegisterAllocator>,

    /// Variable definitions within this scope.
    pub variables: IndexMap<PicoStr, Variable>,

    /// Captured values within this scope.
    pub captures: IndexMap<PicoStr, Capture>,
}

impl<'lib> Scope<'lib> {
    /// Creates a new scope.
    pub fn new(
        is_function: bool,
        is_loop: bool,
        global: Option<&'lib Library>,
        parent: Option<Rc<RefCell<Scope<'lib>>>>,
        capturing: Option<Rc<RefCell<Scope<'lib>>>>,
        registers: Option<RegisterAllocator>,
    ) -> Self {
        Self {
            flags: ScopeFlags::NONE
                | if is_function { ScopeFlags::IN_FUNCTION } else { ScopeFlags::NONE }
                | if is_loop { ScopeFlags::IN_LOOP } else { ScopeFlags::NONE },
            global,
            registers,
            parent,
            capturing,
            captures: IndexMap::new(),
            variables: IndexMap::new(),
        }
    }

    /// Whether we are in a function.
    pub fn in_function(&self) -> bool {
        self.flags.contains(ScopeFlags::IN_FUNCTION)
    }

    /// Whether we are in a loop.
    pub fn in_loop(&self) -> bool {
        self.flags.contains(ScopeFlags::IN_LOOP)
    }

    /// Get access to the global library.
    pub fn global(&self) -> Option<&'lib Library> {
        self.global
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.borrow().global()))
    }

    /// Get access to the register allocator.
    pub fn registers(&self) -> Option<RegisterAllocator> {
        self.registers.clone().or_else(|| {
            self.parent.as_ref().and_then(|parent| parent.borrow().registers())
        })
    }

    /// Allocates a register.
    pub fn allocate(&self) -> RegisterGuard {
        self.registers()
            .expect("tried allocating register with no allocator")
            .allocate()
    }

    /// Allocates a pristine register.
    pub fn allocate_pristine(&self) -> PristineRegisterGuard {
        self.registers()
            .expect("tried allocating register with no allocator")
            .allocate_pristine()
    }

    /// Declare a variable to be written to.
    pub fn write(&mut self, name: impl Into<PicoStr>) -> StrResult<()> {
        let name = name.into();
        if let Some(variable) = self.variables.get_mut(&name) {
            variable.constant = false;
            return Ok(());
        }

        // Find the variable in the parent scope.
        if let Some(parent) = &self.parent {
            return (&**parent).borrow_mut().write(name);
        }

        bail!("cannot write to undeclared variable `{}`", name.resolve())
    }

    /// Declare a variable in this scope.
    pub fn declare_to_register(
        &mut self,
        span: Span,
        name: PicoStr,
        register: RegisterGuard,
    ) {
        self.variables.insert(
            name.clone(),
            Variable {
                register: register.clone(),
                default: None,
                span,
                constant: false,
            },
        );
    }

    /// Declare a variable in this scope.
    pub fn declare(
        &mut self,
        span: Span,
        name: PicoStr,
        default: Option<Value>,
    ) -> RegisterGuard {
        let register = self.allocate();
        self.variables.insert(
            name.clone(),
            Variable {
                register: register.clone(),
                constant: default.is_some(),
                default,
                span,
            },
        );
        register
    }

    /// Read a variable from this scope, excluding the global scope.
    pub fn read_local(&self, var: impl Into<PicoStr>) -> Option<ReadableGuard> {
        let var = var.into();
        if let Some(variable) = self.variables.get(&var) {
            Some(variable.register.clone().into())
        } else {
            let mut next = self.parent.clone();
            while let Some(parent) = next {
                let ref_ = parent.borrow();
                if let Some(variable) = ref_.variables.get(&var) {
                    return Some(variable.register.clone().into());
                }

                next = ref_.parent.clone();
            }
            None
        }
    }

    fn read_captured(
        &mut self,
        span: Span,
        var: impl Into<PicoStr>,
    ) -> Option<ReadableGuard> {
        let var = var.into();
        if let Some(capture) = self.captures.get(&var) {
            return Some(ReadableGuard::Captured(Box::new(ReadableGuard::Register(
                capture.register.clone().into(),
            ))));
        }

        if let Some(mut capturing) = self.capturing.as_deref().map(RefCell::borrow_mut) {
            // If we are capturing, we can read from the capturing scope.
            if let Some(readable) = capturing.read_no_global(span, var) {
                let reg = self.allocate_pristine();
                self.captures.insert(
                    var,
                    Capture {
                        name: var,
                        readable: readable.clone(),
                        register: reg.clone(),
                        span,
                    },
                );

                return Some(ReadableGuard::Captured(Box::new(ReadableGuard::Register(
                    reg.into(),
                ))));
            }
        } else {
            // If we are not capturing, we can try and capture from the parent scope.
            let mut next = self.parent.clone();
            while let Some(ancestor) = next {
                let ref_ = (&*ancestor).borrow_mut();
                if let Some(mut capturing) =
                    ref_.capturing.as_deref().map(RefCell::borrow_mut)
                {
                    if let Some(capture) = ref_.captures.get(&var) {
                        return Some(ReadableGuard::Captured(Box::new(
                            ReadableGuard::Register(capture.register.clone().into()),
                        )));
                    }

                    if let Some(readable) = capturing.read_no_global(span, var) {
                        let reg = self.allocate_pristine();
                        self.captures.insert(
                            var,
                            Capture {
                                name: var,
                                readable: readable.clone(),
                                register: reg.clone(),
                                span,
                            },
                        );

                        return Some(ReadableGuard::Captured(Box::new(
                            ReadableGuard::Register(reg.into()),
                        )));
                    }
                }

                next = ref_.parent.clone();
            }
        }

        None
    }

    /// Read a variable from this scope, including the global scope.
    pub fn read(&mut self, span: Span, var: &str) -> Option<ReadableGuard> {
        if let Some(guard) = self.read_local(var) {
            Some(guard)
        } else if let Some(captured) = self.read_captured(span, var) {
            Some(captured)
        } else if let Some(id) = self.global().and_then(|g| g.global.field_index(var)) {
            Some(Global::new(id as u16).into())
        } else {
            None
        }
    }

    /// Read a variable from this scope, including the global scope.
    pub fn read_no_global(
        &mut self,
        span: Span,
        var: impl Into<PicoStr>,
    ) -> Option<ReadableGuard> {
        let var = var.into();
        if let Some(guard) = self.read_local(var) {
            Some(guard)
        } else if let Some(captured) = self.read_captured(span, var) {
            Some(captured)
        } else {
            None
        }
    }

    /// Read a variable from this scope, including the math scope.
    pub fn read_math(&mut self, span: Span, var: &str) -> Option<ReadableGuard> {
        if let Some(variable) = self.read(span, var) {
            Some(variable)
        } else if let Some(id) = self.global().and_then(|g| g.math.field_index(var)) {
            Some(Math::new(id as u16).into())
        } else {
            None
        }
    }

    /// Tries to resolve a variable from this scope.
    pub fn resolve_var(&self, register: &RegisterGuard) -> Option<Variable> {
        self.variables
            .iter()
            .find(|(_, var)| &var.register == register)
            .map(|(_, var)| var.clone())
    }
}

#[derive(Debug)]
pub struct Capture {
    /// The name of the captured value.
    pub name: PicoStr,
    /// The readable this value is stored in (in the capturing scope).
    pub readable: ReadableGuard,
    /// The register in which this capture is stored.
    pub register: PristineRegisterGuard,
    /// The span where the capture occurs.
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Variable {
    /// The register this variable is stored in.
    pub register: RegisterGuard,
    /// The default value of this variable.
    pub default: Option<Value>,
    /// The span where the variable is declared.
    pub span: Span,
    /// Whether the variable is a constant (up to this point).
    pub constant: bool,
}