use std::{cell::RefCell, fmt, rc::Rc};

use ecow::EcoString;

use crate::diag::bail;
use crate::vm::{
    Constant, Global, Math, OptionalReadable, OptionalWritable, Parent, Readable,
    Register, StringId, Writable,
};

/// The table of occupied registers.
pub struct RegisterTable([(bool, bool); 256]);

impl RegisterTable {
    /// Creates a new empty register table.
    pub fn new() -> Self {
        Self([(false, true); 256])
    }

    /// Allocates a register.
    pub fn allocate(&mut self) -> Option<Register> {
        self.0.iter_mut().enumerate().find(|(_, (is_used, _))| !*is_used).map(
            |(index, (is_used, is_pristine))| {
                *is_used = true;
                *is_pristine = false;
                Register::new(index as u16)
            },
        )
    }

    /// Allocates a pristine register.
    pub fn allocate_pristine(&mut self) -> Option<Register> {
        self.0
            .iter_mut()
            .enumerate()
            .find(|(_, (is_used, is_pristine))| !*is_used && *is_pristine)
            .map(|(index, (is_used, is_pristine))| {
                *is_used = true;
                *is_pristine = false;
                Register::new(index as u16)
            })
    }

    /// Frees a register.
    fn free(&mut self, index: Register) {
        self.0[index.as_raw() as usize] = (false, false);
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

#[derive(Clone)]
pub struct ParentGuard {
    /// The parent access id.
    parent: u16,
    /// The register this variable is stored in.
    register: RegisterGuard,
}

impl fmt::Debug for ParentGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}.{}", self.parent, self.register.as_raw())
    }
}

impl ParentGuard {
    /// Creates a new parent guard.
    pub fn new(parent: u16, register: RegisterGuard) -> Self {
        Self { parent, register }
    }

    /// Get the parent access id.
    pub fn as_parent(&self) -> Parent {
        Parent::new(self.parent, self.register.as_raw())
    }

    /// Get this register as a [`Readable`].
    pub fn as_readable(&self) -> Readable {
        self.as_parent().into()
    }

    /// Get this register as a [`Writable`].
    pub fn as_writeable(&self) -> Writable {
        self.as_parent().into()
    }
}

#[derive(Clone, Debug)]
pub enum ReadableGuard {
    Register(RegisterGuard),
    Captured(Box<ReadableGuard>),
    Parent(ParentGuard),
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
        if readable.is_none() {
            Self::None
        } else if readable.is_auto() {
            Self::Auto
        } else if readable.is_reg() {
            Self::Register(RegisterGuard::new(readable.as_reg(), registers))
        } else if readable.is_string() {
            Self::String(readable.as_string())
        } else if readable.is_const() {
            Self::Constant(readable.as_const())
        } else if readable.is_global() {
            Self::Global(readable.as_global())
        } else if readable.is_math() {
            Self::Math(readable.as_math())
        } else if readable.is_bool() {
            Self::Bool(readable.as_bool())
        } else if readable.is_parent() {
            Self::Parent(ParentGuard::new(
                readable.as_parent().scope(),
                RegisterGuard::new(
                    Register::new(readable.as_parent().value()),
                    registers,
                ),
            ))
        } else {
            unreachable!()
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
            ReadableGuard::Parent(parent) => parent.as_readable(),
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

impl From<ParentGuard> for ReadableGuard {
    fn from(parent: ParentGuard) -> Self {
        Self::Parent(parent)
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
    Parent(ParentGuard),
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

impl From<ParentGuard> for WritableGuard {
    fn from(parent: ParentGuard) -> Self {
        Self::Parent(parent)
    }
}

impl Into<Readable> for &WritableGuard {
    fn into(self) -> Readable {
        match self {
            WritableGuard::Register(register) => register.as_readable(),
            WritableGuard::Parent(parent) => parent.as_readable(),
            WritableGuard::Joined => unreachable!(),
        }
    }
}

impl Into<Writable> for &WritableGuard {
    fn into(self) -> Writable {
        match self {
            WritableGuard::Register(register) => register.as_writeable(),
            WritableGuard::Parent(parent) => parent.as_writeable(),
            WritableGuard::Joined => Writable::joined(),
        }
    }
}

impl Into<OptionalReadable> for Option<ReadableGuard> {
    fn into(self) -> OptionalReadable {
        match self {
            Some(readable) => (&readable).into(),
            None => OptionalReadable::none().into(),
        }
    }
}

impl Into<OptionalReadable> for Option<Constant> {
    fn into(self) -> OptionalReadable {
        match self {
            Some(const_) => OptionalReadable::some(Readable::const_(const_)),
            None => OptionalReadable::none().into(),
        }
    }
}

impl Into<OptionalWritable> for Option<WritableGuard> {
    fn into(self) -> OptionalWritable {
        match self {
            Some(readable) => (&readable).into(),
            None => OptionalWritable::none().into(),
        }
    }
}

impl TryFrom<ReadableGuard> for WritableGuard {
    type Error = EcoString;

    fn try_from(value: ReadableGuard) -> Result<Self, Self::Error> {
        if let ReadableGuard::Register(register) = value {
            Ok(Self::Register(register))
        } else if let ReadableGuard::Parent(parent) = value {
            Ok(Self::Parent(parent))
        } else {
            bail!("this value is not writable")
        }
    }
}
