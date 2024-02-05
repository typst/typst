use std::{cell::RefCell, fmt, rc::Rc};

use ecow::EcoString;

use crate::diag::bail;
use crate::vm::{Constant, Global, Math, Readable, Register, StringId, Writable};

/// The table of occupied registers.
pub struct RegisterTable(Vec<(bool, bool)>);

impl RegisterTable {
    /// Creates a new empty register table.
    pub fn new() -> Self {
        Self(Vec::with_capacity(64))
    }

    /// Allocates a register.
    pub fn allocate(&mut self) -> Register {
        let Some(reg) =
            self.0.iter_mut().enumerate().find(|(_, (is_used, _))| !*is_used).map(
                |(index, (is_used, is_pristine))| {
                    *is_used = true;
                    *is_pristine = false;
                    Register::new(index as u16)
                },
            )
        else {
            return self.allocate_pristine();
        };

        reg
    }

    /// Allocates a pristine register.
    pub fn allocate_pristine(&mut self) -> Register {
        let idx = self.0.len();
        self.0.push((true, false));
        Register::new(idx as u16)
    }

    /// Frees a register.
    fn free(&mut self, index: Register) {
        self.0[index.as_raw() as usize] = (false, false);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// A RAII guard for a register.
///
/// When dropped, the register is freed.
#[derive(Clone)]
pub struct RegisterGuard(Rc<RegisterInner>);

struct RegisterInner(Register, Rc<RefCell<RegisterTable>>);

impl RegisterGuard {
    /// Get the raw index of this register.
    pub fn as_raw(&self) -> u16 {
        self.0 .0.as_raw()
    }

    /// Create a new register guard.
    pub fn new(index: Register, table: Rc<RefCell<RegisterTable>>) -> Self {
        Self(Rc::new(RegisterInner(index, table)))
    }

    /// Get this register as a [`Register`].
    pub fn as_register(&self) -> Register {
        self.0 .0
    }

    /// Get this register as a [`Readable`].
    pub fn as_readable(&self) -> Readable {
        self.as_register().into()
    }

    /// Get this register as a [`Writable`].
    pub fn as_writeable(&self) -> Writable {
        self.as_register().into()
    }
}

impl fmt::Debug for RegisterGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0 .0.fmt(f)
    }
}

impl Drop for RegisterInner {
    fn drop(&mut self) {
        self.1.borrow_mut().free(self.0);
    }
}

#[derive(Clone, Debug)]
pub enum ReadableGuard {
    Register(RegisterGuard),
    Captured(Box<ReadableGuard>),
    Constant(Constant),
    String(StringId),
    Global(Global),
    Math(Math),
    Bool(bool),
    None,
    Auto,
}

impl ReadableGuard {
    pub fn new(readable: Readable, registers: Rc<RefCell<RegisterTable>>) -> Self {
        match readable {
            Readable::Const(constant) => Self::Constant(constant),
            Readable::Reg(reg) => Self::Register(RegisterGuard::new(reg, registers)),
            Readable::Str(string) => Self::String(string),
            Readable::Global(global) => Self::Global(global),
            Readable::Math(math) => Self::Math(math),
            Readable::None => Self::None,
            Readable::Auto => Self::Auto,
            Readable::Bool(value) => Self::Bool(value),
        }
    }
    pub fn as_readable(&self) -> Readable {
        self.into()
    }
}

impl Into<Readable> for &ReadableGuard {
    fn into(self) -> Readable {
        match self {
            ReadableGuard::Register(register) => register.as_readable(),
            ReadableGuard::Captured(captured) => (&**captured).into(),
            ReadableGuard::Constant(constant) => (*constant).into(),
            ReadableGuard::String(string) => (*string).into(),
            ReadableGuard::Global(global) => (*global).into(),
            ReadableGuard::Math(math) => (*math).into(),
            ReadableGuard::None => Readable::none(),
            ReadableGuard::Auto => Readable::auto(),
            ReadableGuard::Bool(value) => Readable::bool(*value),
        }
    }
}

impl From<RegisterGuard> for ReadableGuard {
    fn from(register: RegisterGuard) -> Self {
        Self::Register(register)
    }
}

impl From<Constant> for ReadableGuard {
    fn from(constant: Constant) -> Self {
        Self::Constant(constant)
    }
}

impl From<StringId> for ReadableGuard {
    fn from(string: StringId) -> Self {
        Self::String(string)
    }
}

impl From<Global> for ReadableGuard {
    fn from(global: Global) -> Self {
        Self::Global(global)
    }
}

impl From<Math> for ReadableGuard {
    fn from(math: Math) -> Self {
        Self::Math(math)
    }
}

impl From<bool> for ReadableGuard {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Debug)]
pub enum WritableGuard {
    Register(RegisterGuard),
    Joined,
}

impl WritableGuard {
    pub fn as_writable(&self) -> Writable {
        self.into()
    }

    pub fn is_joined(&self) -> bool {
        matches!(self, Self::Joined)
    }
}

impl From<RegisterGuard> for WritableGuard {
    fn from(register: RegisterGuard) -> Self {
        Self::Register(register)
    }
}

impl Into<Readable> for &WritableGuard {
    fn into(self) -> Readable {
        match self {
            WritableGuard::Register(register) => register.as_readable(),
            WritableGuard::Joined => unreachable!(),
        }
    }
}

impl Into<Writable> for &WritableGuard {
    fn into(self) -> Writable {
        match self {
            WritableGuard::Register(register) => register.as_writeable(),
            WritableGuard::Joined => Writable::joined(),
        }
    }
}

impl TryFrom<ReadableGuard> for WritableGuard {
    type Error = EcoString;

    fn try_from(value: ReadableGuard) -> Result<Self, Self::Error> {
        if let ReadableGuard::Register(register) = value {
            Ok(Self::Register(register))
        } else {
            bail!("this value is not writable")
        }
    }
}
