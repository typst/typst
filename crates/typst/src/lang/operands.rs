use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Readable {
    Const(Constant),
    Reg(Register),
    Str(StringId),
    Global(Global),
    Math(Math),
    Bool(bool),
    Label(LabelId),
    Access(AccessId),
    None,
    Auto,
}

impl Readable {
    /// Creates a new none readable.
    pub const fn none() -> Self {
        Self::None
    }

    /// Creates a new auto readable.
    pub const fn auto() -> Self {
        Self::Auto
    }

    /// Creates a new bool readable.
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Creates a new constant readable.
    pub const fn const_(const_: Constant) -> Self {
        Self::Const(const_)
    }

    /// Creates a new register readable.
    pub const fn reg(reg: Register) -> Self {
        Self::Reg(reg)
    }

    /// Creates a new string readable.
    pub const fn string(string: StringId) -> Self {
        Self::Str(string)
    }

    /// Creates a new global readable.
    pub const fn global(global: Global) -> Self {
        Self::Global(global)
    }

    /// Creates a new math readable.
    pub const fn math(math: Math) -> Self {
        Self::Math(math)
    }

    /// Creates a new access readable.
    pub const fn access(access: AccessId) -> Self {
        Self::Access(access)
    }

    /// Returns this readable as a constant.
    ///
    /// # Panics
    /// Panics if the readable is not a constant.
    pub fn as_const(self) -> Constant {
        match self {
            Self::Const(constant) => constant,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a register.
    ///
    /// # Panics
    /// Panics if the readable is not a register.
    pub fn as_reg(self) -> Register {
        match self {
            Self::Reg(register) => register,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a string.
    ///
    /// # Panics
    /// Panics if the readable is not a string.
    pub fn as_string(self) -> StringId {
        match self {
            Self::Str(string) => string,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a global.
    ///
    /// # Panics
    /// Panics if the readable is not a global.
    pub fn as_global(self) -> Global {
        match self {
            Self::Global(global) => global,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a math.
    ///
    /// # Panics
    /// Panics if the readable is not a math.
    pub fn as_math(self) -> Math {
        match self {
            Self::Math(math) => math,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a bool.
    ///
    /// # Panics
    /// Panics if the readable is not a bool.
    pub fn as_bool(self) -> bool {
        match self {
            Self::Bool(value) => value,
            _ => unreachable!(),
        }
    }

    /// Returns this readable as a label.
    ///
    /// # Panics
    /// Panics if the readable is not a label.
    pub fn as_label(self) -> LabelId {
        match self {
            Self::Label(label) => label,
            _ => unreachable!(),
        }
    }

    /// Returns true if the operand is a register.
    pub fn is_reg(self) -> bool {
        matches!(self, Self::Reg(_))
    }
}

impl fmt::Debug for Readable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(const_) => const_.fmt(f),
            Self::Reg(reg) => reg.fmt(f),
            Self::Str(string) => string.fmt(f),
            Self::Global(global) => global.fmt(f),
            Self::Math(math) => math.fmt(f),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Label(label) => label.fmt(f),
            Self::Access(access) => access.fmt(f),
            Self::None => write!(f, "none"),
            Self::Auto => write!(f, "auto"),
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

impl From<bool> for Readable {
    fn from(value: bool) -> Self {
        Self::bool(value)
    }
}

impl From<LabelId> for Readable {
    fn from(label: LabelId) -> Self {
        Self::Label(label)
    }
}

impl From<AccessId> for Readable {
    fn from(access: AccessId) -> Self {
        Self::Access(access)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Writable {
    Reg(Register),
    Joiner,
}

impl Writable {
    /// Creates a new register writable.
    pub const fn reg(reg: Register) -> Self {
        Self::Reg(reg)
    }

    /// Creates a new joiner writable.
    pub const fn joiner() -> Self {
        Self::Joiner
    }

    /// Returns this writable as a register.
    pub const fn as_reg(self) -> Register {
        match self {
            Self::Reg(register) => register,
            _ => unreachable!(),
        }
    }
}

impl fmt::Debug for Writable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reg(reg) => reg.fmt(f),
            Self::Joiner => write!(f, "J"),
        }
    }
}

impl From<Register> for Writable {
    fn from(register: Register) -> Self {
        Self::reg(register)
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
    ModuleId => "O",
}
