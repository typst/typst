use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ecow::EcoString;
use typst_syntax::Span;

use crate::diag::{bail, error, StrResult};
use crate::foundations::Value;
use crate::vm::{Global, Math};
use crate::Library;

use super::{ParentGuard, ReadableGuard, RegisterGuard, RegisterTable, WritableGuard};

pub struct CompilerScope {
    /// The global library.
    pub global: Library,
    /// The table of occupied registers.
    pub registers: Rc<RefCell<RegisterTable>>,
    /// The parent scope.
    pub parent: Option<Rc<RefCell<Self>>>,
    /// The capturing scopes.
    pub capturing: Option<Rc<RefCell<Self>>>,
    /// The captures (as defined in the capturing scope)
    pub captures: HashMap<EcoString, Capture>,
    /// The table of variables.
    pub variables: HashMap<EcoString, Variable>,
    /// Whether this scope is a function.
    pub is_function: bool,
    /// Whether this scope is a loop.
    pub is_loop: bool,
}

impl CompilerScope {
    pub fn new(global: Library, is_function: bool, is_loop: bool) -> Self {
        Self {
            global,
            registers: Rc::new(RefCell::new(RegisterTable::new())),
            parent: None,
            capturing: None,
            captures: HashMap::new(),
            variables: HashMap::new(),
            is_function,
            is_loop,
        }
    }

    fn with_parent(mut self, parent: Rc<RefCell<Self>>) -> Self {
        self.parent = Some(parent);
        self
    }

    fn with_capture(mut self, capturing: Rc<RefCell<Self>>) -> Self {
        self.capturing = Some(capturing);
        self
    }

    pub fn module(library: Library) -> Self {
        Self::new(library, false, false)
    }

    pub fn function(parent: Rc<RefCell<Self>>) -> Self {
        let global = parent.borrow().global.clone();
        Self::new(global, true, false).with_capture(parent)
    }

    pub fn loop_(parent: Rc<RefCell<Self>>) -> Self {
        let is_function = parent.borrow().is_function;
        let global = parent.borrow().global.clone();
        Self::new(global, is_function, true).with_parent(parent)
    }

    pub fn scope(parent: Rc<RefCell<Self>>, is_loop: bool) -> Self {
        let is_function = parent.borrow().is_function;
        let is_loop = is_loop || parent.borrow().is_loop;
        let global = parent.borrow().global.clone();
        Self::new(global, is_function, is_loop).with_parent(parent)
    }

    pub fn global(&self) -> &Library {
        &self.global
    }

    /// Whether we are in a function.
    pub fn in_function(&self) -> bool {
        self.is_function
            || self.parent.as_ref().map_or(false, |p| p.borrow().in_function())
    }

    /// Whether we are in a loop.
    pub fn in_loop(&self) -> bool {
        self.is_loop || self.parent.as_ref().map_or(false, |p| p.borrow().in_loop())
    }

    /// Allocates a new register.
    pub fn register(&self) -> StrResult<RegisterGuard> {
        let registers = self.registers.clone();
        self.registers
            .borrow_mut()
            .allocate()
            .map(|index| RegisterGuard::new(index, registers))
            .ok_or_else(|| error!("out of registers"))
    }

    /// Allocates a new pristine register.
    pub fn pristine_register(&self) -> StrResult<RegisterGuard> {
        let registers = self.registers.clone();
        self.registers
            .borrow_mut()
            .allocate_pristine()
            .map(|index| RegisterGuard::new(index, registers))
            .ok_or_else(|| error!("out of pristine registers"))
    }

    /// Declare a variable in this scope.
    pub fn declare(&mut self, span: Span, name: EcoString) -> StrResult<RegisterGuard> {
        let register = self.register()?;
        let variable = Variable { register: register.clone(), span, default: None };

        self.variables.insert(name, variable);

        Ok(register)
    }

    /// Declare a variable in this scope.
    pub fn declare_into(
        &mut self,
        span: Span,
        name: EcoString,
        register: RegisterGuard,
    ) -> StrResult<()> {
        let variable = Variable { register, span, default: None };

        self.variables.insert(name, variable);
        Ok(())
    }

    /// Declare a variable in this scope.
    pub fn declare_with_default(
        &mut self,
        span: Span,
        name: EcoString,
        default: Value,
    ) -> StrResult<RegisterGuard> {
        let register = self.pristine_register()?;
        let variable = Variable {
            register: register.clone(),
            span,
            default: Some(default),
        };

        self.variables.insert(name, variable);

        Ok(register)
    }

    /// Read the default value of a variable.
    pub fn default(&self, var: &str) -> StrResult<Option<Value>> {
        if let Some(variable) = self.variables.get(var) {
            Ok(variable.default.clone())
        } else {
            let mut next = self.parent.clone();
            while let Some(parent) = next {
                let ref_ = parent.borrow();
                if let Some(variable) = ref_.variables.get(var) {
                    return Ok(variable.default.clone());
                }

                next = ref_.parent.clone();
            }

            if let Ok(field) = self.global.global.field(var) {
                Ok(Some(field.clone()))
            } else {
                bail!("unknown variable `{}`", var);
            }
        }
    }

    /// Read a variable from this scope, excluding the global scope.
    fn read_own(&self, var: &str) -> Option<ReadableGuard> {
        if let Some(variable) = self.variables.get(var) {
            Some(variable.register.clone().into())
        } else {
            let mut i = 0;
            let mut next = self.parent.clone();
            while let Some(parent) = next {
                let ref_ = parent.borrow();
                if let Some(variable) = ref_.variables.get(var) {
                    return Some(ParentGuard::new(i, variable.register.clone()).into());
                }

                i += 1;
                next = ref_.parent.clone();
            }
            None
        }
    }

    fn read_captured(
        &mut self,
        span: Span,
        var: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        if let Some(capture) = self.captures.get(var) {
            return Ok(Some(ReadableGuard::Captured(Box::new(ReadableGuard::Register(
                capture.register.clone(),
            )))));
        }

        if let Some(capture) = self.capturing.as_ref() {
            let mut ref_ = capture.borrow_mut();
            if let Some(readable) = ref_.read_no_global(span, var)? {
                let reg = self.pristine_register()?;
                self.captures.insert(
                    var.clone(),
                    Capture {
                        name: var.clone(),
                        readable: readable.clone(),
                        register: reg.clone(),
                        span,
                    },
                );
                return Ok(Some(ReadableGuard::Captured(Box::new(
                    ReadableGuard::Register(reg),
                ))));
            }

            Ok(None)
        } else {
            let mut i = 0;
            let mut next = self.parent.clone();
            while let Some(parent) = next {
                let mut ancestor = parent.borrow_mut();
                if let Some(capture) = ancestor.capturing.clone() {
                    if let Some(capture) = ancestor.captures.get(var) {
                        return Ok(Some(ReadableGuard::Captured(Box::new(
                            ReadableGuard::Parent(ParentGuard::new(
                                i,
                                capture.register.clone(),
                            )),
                        ))));
                    }

                    let mut ref_ = capture.borrow_mut();
                    if let Some(readable) = ref_.read_no_global(span, var)? {
                        let reg = ancestor.pristine_register()?;
                        ancestor.captures.insert(
                            var.clone(),
                            Capture {
                                name: var.clone(),
                                readable: readable.clone(),
                                register: reg.clone(),
                                span,
                            },
                        );

                        return Ok(Some(ReadableGuard::Captured(Box::new(
                            ReadableGuard::Parent(ParentGuard::new(i, reg)),
                        ))));
                    }
                }

                i += 1;
                next = ancestor.parent.clone();
            }

            Ok(None)
        }
    }

    /// Read a variable from this scope, excluding the global scope.
    fn write_own(&self, var: &str) -> Option<WritableGuard> {
        if let Some(variable) = self.variables.get(var) {
            Some(variable.register.clone().into())
        } else {
            let mut i = 0;
            let mut next = self.parent.clone();
            while let Some(parent) = next {
                let ref_ = parent.borrow();
                if let Some(variable) = ref_.variables.get(var) {
                    return Some(ParentGuard::new(i, variable.register.clone()).into());
                }

                i += 1;
                next = ref_.parent.clone();
            }
            None
        }
    }

    /// Read a variable from this scope, including the global scope.
    pub fn read(
        &mut self,
        span: Span,
        var: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        if let Some(guard) = self.read_own(var) {
            Ok(Some(guard))
        } else if let Some(captured) = self.read_captured(span, var)? {
            Ok(Some(captured))
        } else if let Ok(id) = self.global.global.field_index(var) {
            Ok(Some(Global::new(id as u16).into()))
        } else {
            Ok(None)
        }
    }

    /// Read a variable from this scope, including the global scope.
    pub fn read_no_global(
        &mut self,
        span: Span,
        var: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        if let Some(guard) = self.read_own(var) {
            Ok(Some(guard))
        } else if let Some(captured) = self.read_captured(span, var)? {
            Ok(Some(captured))
        } else {
            Ok(None)
        }
    }

    /// Read a variable from this scope, including the math scope.
    pub fn read_math(
        &mut self,
        span: Span,
        var: &EcoString,
    ) -> StrResult<Option<ReadableGuard>> {
        if let Some(variable) = self.read(span, var)? {
            Ok(Some(variable))
        } else if let Ok(id) = self.global.math.field_index(var) {
            Ok(Some(Math::new(id as u16).into()))
        } else {
            Ok(None)
        }
    }

    /// Write to a variable from this scope, including the global scope.
    pub fn write(&self, var: &str) -> Option<WritableGuard> {
        self.write_own(var)
    }
}

#[derive(Debug)]
pub struct Variable {
    /// The register this variable is stored in.
    pub register: RegisterGuard,
    /// The default value of this variable.
    pub default: Option<Value>,
    /// The span where the variable is declared.
    pub span: Span,
}

pub struct Capture {
    /// The name of the captured value.
    pub name: EcoString,
    /// The readable this value is stored in (in the parent's scope).
    pub readable: ReadableGuard,
    /// The register in which this capture is stored.
    pub register: RegisterGuard,
    /// The span where the capture occurs.
    pub span: Span,
}
