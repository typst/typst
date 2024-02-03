use std::fmt;

use ecow::{eco_format, EcoString};
use typst_syntax::Span;

use crate::diag::{bail, StrResult};
use crate::foundations::{IntoValue, Label, Value};

use super::{Access, CompiledClosure, Pattern, VMState};

pub trait VmRead {
    type Output<'a>;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>>;
}

impl<T: VmRead> VmRead for Option<T> {
    type Output<'a> = Option<T::Output<'a>>;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        if let Some(this) = self {
            this.read(vm).map(Some)
        } else {
            Ok(None)
        }
    }
}

pub trait VmWrite {
    fn write<'a>(&self, vm: &'a mut VMState) -> StrResult<&'a mut Value>;

    fn write_one(self, vm: &mut VMState, value: impl IntoValue) -> StrResult<()>
    where
        Self: Sized,
    {
        let target = self.write(vm)?;
        *target = value.into_value();

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Readable(u16);

bitflags::bitflags! {
    struct ReadableKind: u16 {
        const NONE     = 0b0000_0000_0000_0000;
        const CONSTANT = 0b0010_0000_0000_0000;
        const REGISTER = 0b0100_0000_0000_0000;
        const STRING   = 0b0110_0000_0000_0000;
        const AUTO     = 0b1000_0000_0000_0000;
        const GLOBAL   = 0b1010_0000_0000_0000;
        const MATH     = 0b1100_0000_0000_0000;
        const BOOL     = 0b0001_0000_0000_0000;
    }
}

impl VmRead for Readable {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        const CONST: u16 = ReadableKind::CONSTANT.bits();
        const REG: u16 = ReadableKind::REGISTER.bits();
        const STR: u16 = ReadableKind::STRING.bits();
        const GLOB: u16 = ReadableKind::GLOBAL.bits();
        const MATH: u16 = ReadableKind::MATH.bits();
        const NONE: u16 = ReadableKind::NONE.bits();
        const AUTO: u16 = ReadableKind::AUTO.bits();
        const BOOL: u16 = ReadableKind::BOOL.bits();

        match self.0 & 0xF000 {
            CONST => self.as_const().read(vm),
            REG => self.as_reg().read(vm),
            STR => self.as_string().read(vm),
            GLOB => self.as_global().read(vm),
            MATH => self.as_math().read(vm),
            NONE => Ok(&Value::None),
            AUTO => Ok(&Value::Auto),
            BOOL => {
                if self.as_bool() {
                    Ok(&Value::Bool(true))
                } else {
                    Ok(&Value::Bool(false))
                }
            }
            _ => bail!("invalid readable: malformed kind"),
        }
    }
}

impl Readable {
    /// Creates a new none readable.
    pub const fn none() -> Self {
        Self(ReadableKind::NONE.bits())
    }

    /// Creates a new auto readable.
    pub const fn auto() -> Self {
        Self(ReadableKind::AUTO.bits())
    }

    /// Creates a new bool readable.
    pub const fn bool(value: bool) -> Self {
        Self(value as u16 | ReadableKind::BOOL.bits())
    }

    /// Creates a new constant readable.
    pub const fn const_(Constant(index): Constant) -> Self {
        debug_assert!(index < 0x1000);

        Self(index | ReadableKind::CONSTANT.bits())
    }

    /// Creates a new register readable.
    pub const fn reg(Register(index): Register) -> Self {
        debug_assert!(index < 0x1000);

        Self(index | ReadableKind::REGISTER.bits())
    }

    /// Creates a new string readable.
    pub const fn string(StringId(index): StringId) -> Self {
        debug_assert!(index < 0x1000);

        Self(index | ReadableKind::STRING.bits())
    }

    /// Creates a new global readable.
    pub const fn global(Global(index): Global) -> Self {
        debug_assert!(index < 0x1000);

        Self(index | ReadableKind::GLOBAL.bits())
    }

    /// Creates a new math readable.
    pub const fn math(Math(index): Math) -> Self {
        debug_assert!(index < 0x1000);

        Self(index | ReadableKind::MATH.bits())
    }

    /// Returns [`true`] if this readable is a constant.
    pub const fn is_const(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::CONSTANT.bits()
    }

    /// Returns [`true`] if this readable is a register.
    pub const fn is_reg(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::REGISTER.bits()
    }

    /// Returns [`true`] if this readable is a string.
    pub const fn is_string(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::STRING.bits()
    }

    /// Returns [`true`] if this readable is a global.
    pub const fn is_global(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::GLOBAL.bits()
    }

    /// Returns [`true`] if this readable is a math.
    pub const fn is_math(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::MATH.bits()
    }

    /// Returns [`true`] if this readable is a none.
    pub const fn is_none(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::NONE.bits()
    }

    /// Returns [`true`] if this readable is a auto.
    pub const fn is_auto(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::AUTO.bits()
    }

    /// Returns [`true`] if this readable is a bool.
    pub const fn is_bool(self) -> bool {
        (self.0 & 0xF000) == ReadableKind::BOOL.bits()
    }

    /// Returns this readable as a constant.
    pub fn as_const(self) -> Constant {
        debug_assert!(self.is_const());

        Constant(self.0 & 0x1FFF)
    }

    /// Returns this readable as a register.
    pub fn as_reg(self) -> Register {
        debug_assert!(self.is_reg());

        Register(self.0 & 0x1FFF)
    }

    /// Returns this readable as a string.
    pub fn as_string(self) -> StringId {
        debug_assert!(self.is_string());

        StringId(self.0 & 0x1FFF)
    }

    /// Returns this readable as a global.
    pub fn as_global(self) -> Global {
        debug_assert!(self.is_global());

        Global(self.0 & 0x1FFF)
    }

    /// Returns this readable as a math.
    pub fn as_math(self) -> Math {
        debug_assert!(self.is_math());

        Math(self.0 & 0x1FFF)
    }

    /// Returns this readable as a bool.
    pub fn as_bool(self) -> bool {
        debug_assert!(self.is_bool());

        self.0 & 0x1 == 1
    }

    /// Returns this readable as its raw representation.
    pub const fn as_raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for Readable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            write!(f, "none")
        } else if self.is_auto() {
            write!(f, "auto")
        } else if self.is_bool() {
            self.as_bool().fmt(f)
        } else if self.is_const() {
            self.as_const().fmt(f)
        } else if self.is_reg() {
            self.as_reg().fmt(f)
        } else if self.is_string() {
            self.as_string().fmt(f)
        } else if self.is_global() {
            self.as_global().fmt(f)
        } else if self.is_math() {
            self.as_math().fmt(f)
        } else {
            unreachable!()
        }
    }
}

impl From<Constant> for Readable {
    fn from(constant: Constant) -> Self {
        Self::const_(constant)
    }
}

impl From<Register> for Readable {
    fn from(register: Register) -> Self {
        Self::reg(register)
    }
}

impl From<StringId> for Readable {
    fn from(string: StringId) -> Self {
        Self::string(string)
    }
}

impl From<Global> for Readable {
    fn from(global: Global) -> Self {
        Self::global(global)
    }
}

impl From<Math> for Readable {
    fn from(math: Math) -> Self {
        Self::math(math)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Writable(u16);

bitflags::bitflags! {
    struct WritableKind: u16 {
        const REGISTER = 0b0100_0000_0000_0000;
        const JOINED = 0b1000_0000_0000_0000;
    }
}

impl Writable {
    /// Creates a new register writeable.
    pub const fn reg(Register(index): Register) -> Self {
        debug_assert!(index < 0x2000);

        Self(index | WritableKind::REGISTER.bits())
    }

    /// Creates a new joined writeable.
    pub const fn joined() -> Self {
        Self(WritableKind::JOINED.bits())
    }

    /// Returns [`true`] if this writeable is a register.
    pub const fn is_reg(self) -> bool {
        (self.0 & 0xE000) == WritableKind::REGISTER.bits()
    }

    /// Returns [`true`] if this writeable is a joined.
    pub const fn is_joined(self) -> bool {
        (self.0 & 0xE000) == WritableKind::JOINED.bits()
    }

    /// Returns this writeable as a register.
    pub const fn as_reg(self) -> Register {
        assert!(self.is_reg());

        Register(self.0 & 0x1FFF)
    }

    /// Returns this writeable as its raw representation.
    pub const fn as_raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for Writable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_joined() {
            write!(f, "J")
        } else if self.is_reg() {
            self.as_reg().fmt(f)
        } else {
            unreachable!()
        }
    }
}

impl From<Register> for Writable {
    fn from(register: Register) -> Self {
        Self::reg(register)
    }
}

impl VmWrite for Writable {
    fn write<'a>(&self, vm: &'a mut VMState) -> StrResult<&'a mut Value> {
        if self.is_reg() {
            self.as_reg().write(vm)
        } else {
            bail!("cannot get mutable reference to joined value")
        }
    }

    fn write_one(self, vm: &mut VMState, value: impl IntoValue) -> StrResult<()>
    where
        Self: Sized,
    {
        if self.is_joined() {
            vm.join(value)
        } else {
            *self.write(vm)? = value.into_value();
            Ok(())
        }
    }
}

impl VmRead for Writable {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        if self.is_reg() {
            self.as_reg().read(vm)
        } else {
            bail!("cannot get reference to joined value")
        }
    }
}

impl TryInto<Readable> for Writable {
    type Error = EcoString;

    fn try_into(self) -> Result<Readable, Self::Error> {
        if self.is_reg() {
            Ok(Readable::from(self.as_reg()))
        } else {
            bail!("cannot read joined value")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OptionalReadable(u16);

impl OptionalReadable {
    /// Creates a new some readable.
    pub fn some(readable: impl Into<Readable>) -> Self {
        let readable = readable.into();
        debug_assert!(readable.as_raw() != u16::MAX);

        Self(readable.as_raw())
    }

    /// Creates a new none readable.
    pub const fn none() -> Self {
        Self(u16::MAX)
    }

    /// Returns [`true`] if this optional readable is a some.
    pub const fn is_some(self) -> bool {
        self.0 != u16::MAX
    }

    /// Returns [`true`] if this optional readable is a none.
    pub const fn is_none(self) -> bool {
        !self.is_some()
    }

    /// Returns this optional readable as a some readable.
    pub const fn ok(self) -> Option<Readable> {
        if self.is_some() {
            Some(Readable(self.0))
        } else {
            None
        }
    }

    /// Returns the raw representation of this optional readable.
    pub const fn as_raw(self) -> u16 {
        self.0
    }
}

impl<T> From<T> for OptionalReadable
where
    T: Into<Readable>,
{
    fn from(readable: T) -> Self {
        Self::some(readable)
    }
}

impl From<Option<Readable>> for OptionalReadable {
    fn from(readable: Option<Readable>) -> Self {
        if let Some(readable) = readable {
            Self::some(readable)
        } else {
            Self::none()
        }
    }
}

impl Into<Option<Readable>> for OptionalReadable {
    fn into(self) -> Option<Readable> {
        self.ok()
    }
}

impl fmt::Debug for OptionalReadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_some() {
            write!(f, "Some({:?})", self.ok())
        } else {
            write!(f, "None")
        }
    }
}

impl VmRead for OptionalReadable {
    type Output<'a> = Option<&'a Value>;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Option<&'a Value>> {
        if let Some(this) = self.ok() {
            this.read(vm).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OptionalWritable(u16);

impl OptionalWritable {
    /// Creates a new some writable.
    pub fn some(writable: impl Into<Writable>) -> Self {
        Self(writable.into().0)
    }

    /// Creates a new none writable.
    pub const fn none() -> Self {
        Self(u16::MAX)
    }

    /// Returns [`true`] if this optional writable is a some.
    pub const fn is_some(self) -> bool {
        self.0 != u16::MAX
    }

    /// Returns [`true`] if this optional writable is a none.
    pub const fn is_none(self) -> bool {
        !self.is_some()
    }

    /// Returns this optional writable as a some writable.
    pub const fn ok(self) -> Option<Writable> {
        if self.is_some() {
            Some(Writable(self.0))
        } else {
            None
        }
    }

    /// Returns the raw representation of this optional writable.
    pub const fn as_raw(self) -> u16 {
        self.0
    }
}

impl<T> From<T> for OptionalWritable
where
    T: Into<Writable>,
{
    fn from(writable: T) -> Self {
        Self::some(writable)
    }
}

impl From<Option<Writable>> for OptionalWritable {
    fn from(writable: Option<Writable>) -> Self {
        if let Some(writable) = writable {
            Self::some(writable)
        } else {
            Self::none()
        }
    }
}

impl Into<Option<Writable>> for OptionalWritable {
    fn into(self) -> Option<Writable> {
        self.ok()
    }
}

impl fmt::Debug for OptionalWritable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_some() {
            write!(f, "Some({:?})", self.ok())
        } else {
            write!(f, "None")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OptionalRegister(u16);

impl OptionalRegister {
    /// Creates a new some register.
    pub fn some(register: impl Into<Register>) -> Self {
        Self(register.into().0)
    }

    /// Creates a new none register.
    pub const fn none() -> Self {
        Self(u16::MAX)
    }

    /// Returns [`true`] if this optional register is a some.
    pub const fn is_some(self) -> bool {
        self.0 != u16::MAX
    }

    /// Returns [`true`] if this optional register is a none.
    pub const fn is_none(self) -> bool {
        !self.is_some()
    }

    /// Returns this optional register as a some register.
    pub const fn ok(self) -> Option<Register> {
        if self.is_some() {
            Some(Register(self.0))
        } else {
            None
        }
    }
}

impl<T> From<T> for OptionalRegister
where
    T: Into<Register>,
{
    fn from(register: T) -> Self {
        Self::some(register)
    }
}

impl From<Option<Register>> for OptionalRegister {
    fn from(register: Option<Register>) -> Self {
        if let Some(register) = register {
            Self::some(register)
        } else {
            Self::none()
        }
    }
}

impl Into<Option<Register>> for OptionalRegister {
    fn into(self) -> Option<Register> {
        self.ok()
    }
}

impl fmt::Debug for OptionalRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_some() {
            write!(f, "Some({:?})", self.ok())
        } else {
            write!(f, "None")
        }
    }
}

macro_rules! id {
    (
        $(#[$sattr:meta])*
        $name:ident($type:ty) => $l:literal
    ) => {
        $(#[$attr])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub $type);

        impl $name {
            pub fn new(index: $type) -> Self {
                Self(index)
            }

            pub const fn as_raw(self) -> $type {
                self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($l, "{:?}"), self.0)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $name:ident => $l:literal
    ) => {
        $(#[$attr])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub u16);

        impl $name {
            pub const fn new(index: u16) -> Self {
                Self(index)
            }

            pub const fn as_raw(self) -> u16 {
                self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($l, "{}"), self.0)
            }
        }
    };
    ($(
        $(#[$attr:meta])*
        $name:ident$(($type:ty))? => $l:literal
    ),* $(,)*) => {
        $( id!(
            $(#[$attr])*
            $name$(($type))? => $l
        ); )*
    };
}

id! {
    Constant => "C",
    Register => "R",
    StringId => "S",
    ClosureId => "F",
    AccessId => "A",
    Global => "G",
    Math => "M",
    Pointer => "P",
    LabelId => "L",
    CallId => "K",
    PatternId => "D",
    SpanId => "N",
}

impl VmRead for Constant {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        vm.constants.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid constant: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for Register {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        vm.registers.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid register: {}, malformed instruction", self.0)
        })
    }
}

impl VmWrite for Register {
    fn write<'a>(&self, vm: &'a mut VMState) -> StrResult<&'a mut Value> {
        vm.registers.get_mut(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid register: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for StringId {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        vm.strings.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid string: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for ClosureId {
    type Output<'a> = &'a CompiledClosure;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a CompiledClosure> {
        vm.closures.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid closure: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for Global {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        vm.global
            .global
            .field_by_id(self.0 as usize)
            .map_err(|_| eco_format!("invalid global: {}, malformed instruction", self.0))
    }
}

impl VmRead for Math {
    type Output<'a> = &'a Value;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Value> {
        vm.global
            .math
            .field_by_id(self.0 as usize)
            .map_err(|_| eco_format!("invalid math: {}, malformed instruction", self.0))
    }
}

impl VmRead for LabelId {
    type Output<'a> = &'a Label;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<&'a Label> {
        vm.labels.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid label: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for AccessId {
    type Output<'a> = &'a Access;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        vm.accesses.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid access: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for PatternId {
    type Output<'a> = &'a Pattern;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        vm.patterns.get(self.0 as usize).ok_or_else(|| {
            eco_format!("invalid destructure: {}, malformed instruction", self.0)
        })
    }
}

impl VmRead for SpanId {
    type Output<'a> = Span;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        vm.spans
            .get(self.0 as usize)
            .copied()
            .ok_or_else(|| eco_format!("invalid span: {}, malformed instruction", self.0))
    }
}

impl VmRead for Pointer {
    type Output<'a> = usize;

    fn read<'a>(&self, vm: &'a VMState) -> StrResult<Self::Output<'a>> {
        vm.jumps
            .get(self.0 as usize)
            .copied()
            .ok_or_else(|| eco_format!("invalid jump: {}, malformed instruction", self.0))
    }
}
