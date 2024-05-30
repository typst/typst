use std::borrow::Cow;
use std::sync::Arc;

use typst_syntax::Span;
use typst_utils::{LazyHash, PicoStr};

use crate::foundations::Value;

use super::compiled::{CompiledCode, CompiledParam};
use super::compiler::Compiler;
use super::operands::Register;

/// A closure that has been instantiated.
#[derive(Clone, Hash, PartialEq)]
pub struct Closure {
    pub inner: Arc<LazyHash<Repr>>,
}

#[derive(Hash)]
pub struct Repr {
    /// The compiled code of the closure.
    pub compiled: CompiledCode,
    /// The parameters of the closure.
    params: Vec<(Option<Register>, Param)>,
    /// The captured values and where to store them.
    captures: Vec<(Register, Value)>,
}

impl Closure {
    /// Creates a new closure.
    pub fn new(
        compiled: CompiledCode,
        params: Vec<(Option<Register>, Param)>,
        captures: Vec<(Register, Value)>,
    ) -> Closure {
        Self {
            inner: Arc::new(LazyHash::new(Repr { compiled, params, captures })),
        }
    }

    pub fn no_instance(compiled: CompiledCode, compiler: &Compiler) -> Self {
        let params = compiled
            .params
            .iter()
            .flat_map(|params| params.iter())
            .map(|param| match param {
                CompiledParam::Pos(output, name) => (Some(*output), Param::Pos(*name)),
                CompiledParam::Named { target, name, default, .. } => {
                    let Some(default) = default else {
                        return (
                            Some(*target),
                            Param::Named { name: *name, default: None },
                        );
                    };

                    let Some(default) = compiler.resolve(*default).map(Cow::into_owned)
                    else {
                        panic!("default value not resolved, this is a compiler bug.");
                    };

                    (Some(*target), Param::Named { name: *name, default: Some(default) })
                }
                CompiledParam::Sink(span, dest, name) => {
                    (*dest, Param::Sink(*span, *name))
                }
            })
            .collect();

        Self::new(compiled, params, Vec::new())
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum Param {
    /// A positional parameter.
    Pos(PicoStr),
    /// A named parameter.
    Named {
        /// The name of the parameter.
        name: PicoStr,
        /// The default value of the parameter.
        default: Option<Value>,
    },
    /// A sink parameter.
    Sink(Span, PicoStr),
}
