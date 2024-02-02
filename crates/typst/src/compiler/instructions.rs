use std::num::NonZeroU32;

use typst_syntax::Span;

use super::Compiler;
use crate::vm::{
    AccessId, ClosureId, LabelId, OptionalReadable, OptionalWritable, PatternId, Pointer,
    Readable, ScopeId, Writable,
};

pub use crate::vm::opcodes::*;

macro_rules! snek_filter {
    ($(#[$sattr:meta])*
    $name:ident: jump $(-> $out:ty)? $(=> {
        $(
            $(#[$attr:meta])*
            $arg:ident: $arg_ty:ty
        ),* $(,)?
    })?) => {
        snek_filter! {
            $(#[$sattr])*
            $name: jump_isr $(-> $out)? $(=> {
                $(
                    $(#[$attr])*
                    $arg: $arg_ty
                ),*,
            })?
        }
    };
    ($(#[$sattr:meta])*
    $name:ident: enter $(-> $out:ty)? $(=> {
        $(
            $(#[$attr:meta])*
            $arg:ident: $arg_ty:ty
        ),* $(,)?
    })?) => {
        snek_filter! {
            $(#[$sattr])*
            $name: enter_isr $(-> $out)? $(=> {
                $(
                    $(#[$attr])*
                    $arg: $arg_ty
                ),*,
            })?
        }
    };
    (
        $(#[$sattr:meta])*
    $name:ident: $snek:ident $(-> $out:ty)? $(=> {
        $(
            $(#[$attr:meta])*
            $arg:ident: $arg_ty:ty
        ),* $(,)?
    })?) => {
        pub fn $snek(&mut self, span: Span, $($($arg: impl Into<$arg_ty>,)*)? $(out: impl Into<$out>)?) {
            let opcode = crate::vm::opcodes::$name {
                $($(
                    $arg: $arg.into(),
                )*)?
                $(
                    out: <_ as Into<$out>>::into(out),
                )?
            };

            self.spans.push(span);
            self.instructions.push(crate::vm::opcodes::Opcode::$name(opcode));
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
        })?
    ),* $(,)?) => {
        impl Compiler {
            $(
                snek_filter! {
                    $(#[$sattr])*
                    $name: $snek $(-> $out)? $(=> {
                        $(
                            $(#[$attr])*
                            $arg: $arg_ty
                        ),*,
                    })?
                }
            )*
        }
    }
}

include!("../vm/opcodes_raw.rs");
