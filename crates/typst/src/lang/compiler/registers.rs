use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::{cell::RefCell, fmt, rc::Rc};

use ecow::EcoString;

use crate::diag::bail;
use crate::lang::opcodes::AccessId;
use crate::lang::operands::{
    Constant, Global, LabelId, Math, Readable, Register, StringId, Writable,
};

#[derive(Clone)]
pub struct RegisterAllocator {
    table: Rc<RefCell<RegisterTable>>,
}

impl Default for RegisterAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterAllocator {
    /// Creates a new register allocator.
    pub fn new() -> Self {
        Self { table: Rc::new(RefCell::new(RegisterTable::new())) }
    }

    /// Allocates a register.
    pub fn allocate(&self) -> RegisterGuard {
        let reg = self.table.borrow_mut().allocate();
        RegisterGuard::new(reg, self.table.clone())
    }

    /// Allocates a pristine register.
    pub fn allocate_pristine(&self) -> PristineRegisterGuard {
        PristineRegisterGuard(self.allocate())
    }

    /// The number of allocated registers.
    pub fn len(&self) -> usize {
        self.table.borrow().len()
    }
}

/// The table of occupied registers.
pub struct RegisterTable(Vec<bool>);

impl Default for RegisterTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterTable {
    /// Creates a new empty register table.
    pub fn new() -> Self {
        Self(Vec::with_capacity(64))
    }

    /// Allocates a register.
    pub fn allocate(&mut self) -> Register {
        let Some(reg) =
            self.0.iter_mut().enumerate().find(|(_, is_used)| !**is_used).map(
                |(index, is_used)| {
                    *is_used = true;
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
        self.0.push(true);
        Register::new(idx as u16)
    }

    /// Frees a register.
    fn free(&mut self, index: Register) {
        self.0[index.as_raw() as usize] = false;
    }

    /// Gives the number of registers that have been allocated.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Checks if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A RAII guard for a pristine register.
///
/// When dropped, the register is freed.
#[derive(Clone, Debug)]
pub struct PristineRegisterGuard(RegisterGuard);

impl From<PristineRegisterGuard> for RegisterGuard {
    fn from(val: PristineRegisterGuard) -> Self {
        val.0
    }
}

impl From<PristineRegisterGuard> for Register {
    fn from(val: PristineRegisterGuard) -> Self {
        val.0.into()
    }
}

impl Deref for PristineRegisterGuard {
    type Target = RegisterGuard;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A RAII guard for a register.
///
/// When dropped, the register is freed.
#[derive(Clone, Hash, PartialEq)]
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
    pub fn as_writable(&self) -> Writable {
        self.as_register().into()
    }
}

impl PartialEq for RegisterInner {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for RegisterInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl fmt::Debug for RegisterGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0 .0.fmt(f)
    }
}

impl From<RegisterGuard> for Readable {
    fn from(val: RegisterGuard) -> Self {
        val.as_readable()
    }
}

impl From<RegisterGuard> for Writable {
    fn from(val: RegisterGuard) -> Self {
        val.as_writable()
    }
}

impl From<RegisterGuard> for Register {
    fn from(val: RegisterGuard) -> Self {
        val.as_register()
    }
}

impl Drop for RegisterInner {
    fn drop(&mut self) {
        self.1.borrow_mut().free(self.0);
    }
}

#[derive(Clone, Debug, Hash)]
pub enum ReadableGuard {
    Register(RegisterGuard),
    Captured(Box<ReadableGuard>),
    Constant(Constant),
    String(StringId),
    Global(Global),
    Math(Math),
    Bool(bool),
    Label(LabelId),
    Access(AccessId),
    None,
    Auto,
}

impl From<ReadableGuard> for Readable {
    fn from(val: ReadableGuard) -> Self {
        match val {
            ReadableGuard::Register(register) => register.as_readable(),
            ReadableGuard::Captured(captured) => (*captured).into(),
            ReadableGuard::Constant(constant) => constant.into(),
            ReadableGuard::String(string) => string.into(),
            ReadableGuard::Global(global) => global.into(),
            ReadableGuard::Math(math) => math.into(),
            ReadableGuard::None => Readable::none(),
            ReadableGuard::Auto => Readable::auto(),
            ReadableGuard::Bool(value) => Readable::bool(value),
            ReadableGuard::Access(access) => Readable::access(access),
            ReadableGuard::Label(label) => label.into(),
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

impl From<LabelId> for ReadableGuard {
    fn from(label: LabelId) -> Self {
        Self::Label(label)
    }
}

impl From<AccessId> for ReadableGuard {
    fn from(access: AccessId) -> Self {
        Self::Access(access)
    }
}

#[derive(Clone, Debug, Hash)]
pub enum WritableGuard {
    Register(RegisterGuard),
    Joined,
}

impl WritableGuard {
    pub fn as_writable(&self) -> Writable {
        self.clone().into()
    }

    pub fn is_joiner(&self) -> bool {
        matches!(self, Self::Joined)
    }
}

impl From<RegisterGuard> for WritableGuard {
    fn from(register: RegisterGuard) -> Self {
        Self::Register(register)
    }
}

impl From<WritableGuard> for Readable {
    fn from(val: WritableGuard) -> Self {
        match val {
            WritableGuard::Register(register) => register.as_readable(),
            WritableGuard::Joined => unreachable!(),
        }
    }
}

impl From<WritableGuard> for Writable {
    fn from(val: WritableGuard) -> Self {
        match val {
            WritableGuard::Register(register) => register.as_writable(),
            WritableGuard::Joined => Writable::joiner(),
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
