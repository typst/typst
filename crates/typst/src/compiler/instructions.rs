use std::num::NonZeroU32;
use std::{fmt, mem::size_of};

use bytemuck::cast;
use typst_syntax::Span;

use crate::vm::{
    AccessId, ClosureId, LabelId, OptionalReadable, OptionalWritable, PatternId,
    Readable, ScopeId, Writable,
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
        #[derive(Debug, Copy, Clone)]
        #[repr(C)]
        pub struct $name {
            #[doc = "The span of the instruction."]
            pub span: Span,
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

        #[derive(Clone)]
        pub enum Opcode {
            JumpLabel(Option<ScopeId>, JumpLabel),
            Flow,
            $(
                $name($name),
            )*
        }

        impl Opcode {
            pub fn span(&self) -> Span {
                match self {
                    Self::JumpLabel(_, _) => Span::detached(),
                    Self::Flow => Span::detached(),
                    $(
                        Self::$name(isr) => isr.span,
                    )*
                }
            }

            $(
                pub fn $snek(span: Span, $($($arg: impl Into<$arg_ty>,)*)? $(out: impl Into<$out>)?) -> Self {
                    Self::$name($name {
                        span,
                        $($(
                            $arg: $arg.into(),
                        )*)?
                        $(
                            out: <_ as Into<$out>>::into(out),
                        )?
                    })
                }
            )*

            pub fn jump_label(_: Span, scope_id: Option<ScopeId>, label: JumpLabel) -> Self {
                Self::JumpLabel(scope_id, label)
            }
        }

        impl Write for Opcode {
            #[allow(unused_variables)]
            fn write(&self, opcodes: &[Opcode], buffer: &mut Vec<u8>) {
                match self {
                    Self::JumpLabel(_, _) => {},
                    Self::Flow => {
                        buffer.push(0x00);
                    },
                    $(
                        Self::$name(isr) => {
                            buffer.reserve(std::mem::size_of::<$name>());

                            buffer.push($value);
                            buffer.extend(cast::<u64, [u8; 8]>(isr.span.as_raw().get()));
                            $($(
                                isr.$arg.write(opcodes, buffer);
                            )*)?
                            $(<$out as Write>::write(&isr.out, opcodes, buffer);)?
                        }
                    )*
                }
            }

            #[allow(unused_variables)]
            fn size(&self) -> usize {
                match self {
                    Self::JumpLabel(_, _) => 0,
                    Self::Flow => 1,
                    $(
                        Self::$name(isr) => {
                            1 + isr.span.size() $($(
                                + isr.$arg.size()
                            )*)? $(+ <$out as Write>::size(&isr.out))?
                        }
                    )*
                }
            }
        }

        impl fmt::Debug for Opcode {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::JumpLabel(_, i) => i.fmt(f),
                    Self::Flow => write!(f, "Flow"),
                    $(
                        Self::$name(isr) => isr.fmt(f),
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
        fn recursive_find<'a, I: Iterator<Item = &'a Opcode>>(
            to_find: &JumpLabel,
            iter: &mut std::iter::Peekable<I>,
            len: u32,
            current_scope: Option<ScopeId>,
        ) -> Option<u32> {
            let mut i = 0;
            while i != len {
                let opcode = iter.next()?;
                match opcode {
                    Opcode::JumpLabel(scope_id, label) => {
                        if label == to_find {
                            debug_assert!(*scope_id == current_scope);
                            return Some(i);
                        }
                    }
                    Opcode::Enter(op) => {
                        if let Some(i) =
                            recursive_find(to_find, iter, op.len, Some(op.scope))
                        {
                            debug_assert!(i <= op.len);
                            return Some(i);
                        } else {
                            i += op.len;
                        }
                    }
                    Opcode::Iter(op) => {
                        if let Some(i) =
                            recursive_find(to_find, iter, op.len, Some(op.scope))
                        {
                            debug_assert!(i <= op.len);
                            return Some(i);
                        } else {
                            i += op.len;
                        }
                    }
                    Opcode::While(op) => {
                        if let Some(i) =
                            recursive_find(to_find, iter, op.len, Some(op.scope))
                        {
                            debug_assert!(i <= op.len);
                            return Some(i);
                        } else {
                            i += op.len;
                        }
                    }
                    _ => {}
                }

                i += opcode.size() as u32;
            }

            if let Some(Opcode::JumpLabel(id, label)) = iter.peek() {
                if *id == current_scope && label == to_find {
                    return Some(i);
                }
            }

            if len != u32::MAX {
                debug_assert!(i == len);
            }

            None
        }

        let index = recursive_find(self, &mut opcodes.iter().peekable(), u32::MAX, None)
            .expect("Jump label not found");

        debug_assert!(
            index <= opcodes.iter().map(|opcode| opcode.size() as u32).sum::<u32>()
        );

        crate::vm::Pointer::new(index).write(opcodes, buffer);
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
