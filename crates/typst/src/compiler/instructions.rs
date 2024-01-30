use bytemuck::cast;
use std::{mem::size_of, num::NonZeroU32};
use typst_syntax::Span;

use crate::vm::{
    AccessId, ClosureId, LabelId, OptionalReadable, OptionalWritable, PatternId,
    Readable, Writable, ScopeId,
};

type Pointer = JumpLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JumpLabel(pub u16);

macro_rules! opcode_struct {
    (
        $(#[$sattr:meta])*
        $name:ident $(-> $out:ty)? $(=> {
            $(
                $(#[$attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?
        })?
    ) => {
        $(#[$sattr])*
        #[derive(Debug, Clone)]
        pub struct $name {
            $(
                $(
                    $(#[$attr])*
                    pub $arg: $arg_ty,
                )*
            )?
            $(
                #[doc = "The output of the instruction."]
                pub out: $out,
            )?
        }
    };
}

macro_rules! opcodes {
    ($(
        $(#[$sattr:meta])*
        $name:ident: $snek:ident $(-> $out:ty)? $(=> {
            $(
                $(#[$attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?
        })? = $value:expr
    ),* $(,)?) => {
        $(
            opcode_struct! {
                $(#[$sattr])*
                $name $(-> $out)? $(=> {
                    $(
                        $(#[$attr])*
                        $arg: $arg_ty
                    ),*
                })?
            }
        )*

        #[derive(Debug, Clone)]
        pub enum Opcode {
            JumpLabel(JumpLabel),
            $(
                $name(Span, $name),
            )*
        }

        impl Opcode {
            pub fn span(&self) -> Span {
                match self {
                    Self::JumpLabel(_) => Span::detached(),
                    $(
                        Self::$name(span, _) => *span,
                    )*
                }
            }

            $(
                pub fn $snek(span: Span, $($($arg: impl Into<$arg_ty>,)*)? $(out: impl Into<$out>)?) -> Self {
                    Self::$name(span, $name {
                        $($(
                            $arg: $arg.into(),
                        )*)?
                        $(
                            out: <_ as Into<$out>>::into(out),
                        )?
                    })
                }
            )*

            pub fn jump_label(_: Span, label: JumpLabel) -> Self {
                Self::JumpLabel(label)
            }
        }

        impl Write for Opcode {
            #[allow(unused_variables)]
            fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
                match self {
                    Self::JumpLabel(_) => {},
                    $(
                        Self::$name(span, isr) => {
                            buffer.push($value);
                            span.write(opcodes, buffer);
                            $(
                                <$out as Write>::write(&isr.out, opcodes, buffer);
                            )?
                            $(
                                $(
                                    isr.$arg.write(opcodes, buffer);
                                )*
                            )?
                        }
                    )*
                }
            }

            #[allow(unused_variables)]
            fn size(&self) -> usize {
                match self {
                    Self::JumpLabel(_) => 0,
                    $(
                        Self::$name(span, isr) => {
                            1 + span.size() $(
                                + <$out as Write>::size(&isr.out)
                            )? $(
                                $(
                                    + <$arg_ty as Write>::size(&isr.$arg)
                                )*
                            )?
                        }
                    )*
                }
            }
        }
    }
}

include!("../vm/opcodes_raw.rs");

pub trait Write {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>);

    fn size(&self) -> usize;
}

impl Write for Span {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().get().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<Span>()
    }
}

impl Write for u64 {
    fn write(&self, _: &[Opcode], buffer: &mut Vec<u8>) {
        buffer.extend(cast::<u64, [u8; 8]>(*self));
    }

    fn size(&self) -> usize {
        size_of::<u64>()
    }
}

impl Write for u32 {
    fn write(&self, _: &[Opcode], buffer: &mut Vec<u8>) {
        buffer.extend(cast::<u32, [u8; 4]>(*self));
    }

    fn size(&self) -> usize {
        size_of::<u32>()
    }
}

impl Write for u16 {
    fn write(&self, _: &[Opcode], buffer: &mut Vec<u8>) {
        buffer.extend(cast::<u16, [u8; 2]>(*self));
    }

    fn size(&self) -> usize {
        size_of::<u16>()
    }
}

impl Write for u8 {
    fn write(&self, _: &[Opcode], buffer: &mut Vec<u8>) {
        buffer.push(*self);
    }

    fn size(&self) -> usize {
        size_of::<u8>()
    }
}

impl Write for Option<NonZeroU32> {
    fn write(&self, _: &[Opcode], buffer: &mut Vec<u8>) {
        buffer.extend(cast::<Option<NonZeroU32>, [u8; 4]>(*self));
    }

    fn size(&self) -> usize {
        size_of::<Option<NonZeroU32>>()
    }
}

impl Write for Writable {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        self.as_raw().size()
    }
}

impl Write for OptionalWritable {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<u16>()
    }
}

impl Write for Readable {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        self.as_raw().size()
    }
}

impl Write for OptionalReadable {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<u16>()
    }
}

impl Write for AccessId {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<AccessId>()
    }
}

impl Write for ClosureId {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<ClosureId>()
    }
}

impl Write for LabelId {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<LabelId>()
    }
}

impl Write for PatternId {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<PatternId>()
    }
}

impl Write for JumpLabel {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        opcodes
            .iter()
            .position(|opcode| match opcode {
                Opcode::JumpLabel(label) => label == self,
                _ => false,
            })
            .map(|i| crate::vm::Pointer::new(i as u32))
            .unwrap()
            .write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<crate::vm::Pointer>()
    }
}

impl Write for crate::vm::Pointer {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<crate::vm::Pointer>()
    }
}

impl Write for ScopeId {
    fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
        self.as_raw().write(opcodes, buffer);
    }

    fn size(&self) -> usize {
        size_of::<ScopeId>()
    }
}
