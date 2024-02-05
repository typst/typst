use std::collections::HashSet;

use ecow::EcoString;
use smallvec::SmallVec;
use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::foundations::{Array, Dict, Value};

use super::{Access, VMState};

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct Pattern {
    pub span: Span,
    pub kind: PatternKind,
}

impl Pattern {
    pub fn write(&self, vm: &mut VMState, value: Value) -> SourceResult<()> {
        match &self.kind {
            PatternKind::Single(single) => match single {
                // Placeholders simply discard the value.
                PatternItem::Placeholder(_) => {}
                PatternItem::Simple(span, local, _) => {
                    local.write(*span, vm)?;
                }
                PatternItem::Named(span, _, _) => bail!(
                    *span,
                    "cannot destructure {} with named pattern",
                    value.ty().long_name()
                ),
                PatternItem::Spread(span, _) | PatternItem::SpreadDiscard(span) => bail!(
                    *span,
                    "cannot destructure {} with spread",
                    value.ty().long_name()
                ),
            },
            PatternKind::Tuple(tuple, has_sink) => match value {
                Value::Array(array) => destructure_array(vm, array, tuple)?,
                Value::Dict(dict) => destructure_dict(vm, dict, *has_sink, tuple)?,
                other => {
                    bail!(self.span, "cannot destructure {}", other.ty().long_name())
                }
            },
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum PatternKind {
    /// Destructure into a single local.
    Single(PatternItem),

    /// Destructure into a tuple of locals.
    Tuple(SmallVec<[PatternItem; 2]>, bool),
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum PatternItem {
    /// Destructure into a single local.
    Placeholder(Span),

    /// Destructure into a single local.
    Simple(Span, Access, EcoString),

    /// Spread the remaining values into a single value.
    Spread(Span, Access),

    /// Spread the remaining values into a single value and discard it.
    SpreadDiscard(Span),

    /// A named pattern.
    Named(Span, Access, EcoString),
}

fn destructure_array(
    vm: &mut VMState,
    value: Array,
    tuple: &[PatternItem],
) -> SourceResult<()> {
    let mut i = 0;
    let len = value.as_slice().len();
    for p in tuple {
        match p {
            PatternItem::Named(span, _, _) => {
                bail!(*span, "cannot destructure array with named pattern")
            }
            PatternItem::Placeholder(span) => {
                if i < len {
                    i += 1
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::Simple(span, local, _) => {
                if i < len {
                    *local.write(*span, vm)? = value.as_slice()[i].clone();
                    i += 1;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::Spread(span, local) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                if let (Some(sink_size), Some(sink)) = (sink_size, sink) {
                    *local.write(*span, vm)? = Value::Array(sink.into());
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            PatternItem::SpreadDiscard(span) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                if let (Some(sink_size), Some(_)) = (sink_size, sink) {
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
        }
    }

    Ok(())
}

fn destructure_dict(
    vm: &mut VMState,
    dict: Dict,
    has_sink: bool,
    tuple: &[PatternItem],
) -> SourceResult<()> {
    // If there is no sink, we purposefully don't bother allocating
    // a set for the used keys.
    let mut sink = None;
    let mut used = has_sink.then(HashSet::new);

    for p in tuple {
        match p {
            PatternItem::Simple(span, local, name) => {
                let v = dict.get(&name).at(*span)?;
                *local.write(*span, vm)? = v.clone();

                used.as_mut().map(|u| u.insert(name.clone()));
            }
            PatternItem::Placeholder(_) => {}
            PatternItem::Spread(span, local) => sink = Some((*span, Some(local))),
            PatternItem::SpreadDiscard(span) => sink = Some((*span, None)),
            PatternItem::Named(span, local, name) => {
                let v = dict.get(&name).at(*span)?;
                *local.write(*span, vm)? = v.clone();
                used.as_mut().map(|u| u.insert(name.clone()));
            }
        }
    }

    if let Some((span, local)) = sink {
        let used = used.unwrap();
        if let Some(local) = local {
            let mut sink = Dict::new();
            for (key, value) in dict {
                if !used.contains(key.as_str()) {
                    sink.insert(key, value);
                }
            }

            *local.write(span, vm)? = Value::Dict(sink);
        }
    }

    Ok(())
}
