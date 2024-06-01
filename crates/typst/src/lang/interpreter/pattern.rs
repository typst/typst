use std::collections::HashSet;

use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Array, Dict, Value};
use crate::lang::compiled::{CompiledPattern, CompiledPatternItem, CompiledPatternKind};

use super::Vm;

impl CompiledPattern {
    pub fn write(
        &self,
        vm: &mut Vm,
        engine: &mut Engine,
        value: Value,
    ) -> SourceResult<()> {
        match &self.kind {
            CompiledPatternKind::Single(single) => match single {
                // Placeholders simply discards the value.
                CompiledPatternItem::Placeholder(_) => {}
                CompiledPatternItem::Simple(span, local_id, _) => {
                    let access = vm.read(*local_id);
                    access.write(*span, vm, engine)?;
                }
                CompiledPatternItem::Named(span, _, _) => bail!(
                    *span,
                    "cannot destructure {} with named pattern",
                    value.ty().long_name()
                ),
                CompiledPatternItem::Spread(span, _)
                | CompiledPatternItem::SpreadDiscard(span) => bail!(
                    *span,
                    "cannot destructure {} with spread",
                    value.ty().long_name()
                ),
                CompiledPatternItem::Nested(_, pattern_id) => {
                    let pattern = vm.read(*pattern_id);
                    pattern.write(vm, engine, value)?;
                }
            },
            CompiledPatternKind::Tuple(tuple, has_sink) => match value {
                Value::Array(array) => destructure_array(vm, engine, array, tuple)?,
                Value::Dict(dict) => {
                    destructure_dict(vm, engine, dict, *has_sink, tuple)?
                }
                other => {
                    bail!(self.span, "cannot destructure {}", other.ty().long_name())
                }
            },
        }

        Ok(())
    }
}

/// Perform destructuring on an array.
fn destructure_array(
    vm: &mut Vm,
    engine: &mut Engine,
    value: Array,
    tuple: &[CompiledPatternItem],
) -> SourceResult<()> {
    let mut i = 0;
    let len = value.as_slice().len();

    let check_len = |i: usize, span: Span| {
        if i < len {
            bail!(span, "not enough elements to destructure")
        }
        Ok(())
    };

    for p in tuple {
        match p {
            CompiledPatternItem::Placeholder(span) => {
                check_len(i, *span)?;
            }
            CompiledPatternItem::Simple(span, access, _) => {
                check_len(i, *span)?;

                // Resolve the access and write the value.
                let access = vm.read(*access);
                let location = access.write(*span, vm, engine)?;

                *location = value.as_slice()[i].clone();
                i += 1;
            }
            CompiledPatternItem::Nested(span, nested_id) => {
                check_len(i, *span)?;

                let nested = vm.read(*nested_id);
                nested.write(vm, engine, value.as_slice()[i].clone())?;
                i += 1;
            }
            CompiledPatternItem::Spread(span, access_id) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));

                if let (Some(sink_size), Some(sink)) = (sink_size, sink) {
                    let access = vm.read(*access_id);
                    let location = access.write(*span, vm, engine)?;

                    *location = Value::Array(Array::from(sink));
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            CompiledPatternItem::SpreadDiscard(span) => {
                let sink_size = (1 + len).checked_sub(tuple.len());
                let sink = sink_size.and_then(|s| value.as_slice().get(i..i + s));
                if let (Some(sink_size), Some(_)) = (sink_size, sink) {
                    i += sink_size;
                } else {
                    bail!(*span, "not enough elements to destructure")
                }
            }
            CompiledPatternItem::Named(span, _, _) => {
                bail!(*span, "cannot destructure array with named pattern")
            }
        }
    }

    Ok(())
}

fn destructure_dict(
    vm: &mut Vm,
    engine: &mut Engine,
    dict: Dict,
    has_sink: bool,
    tuple: &[CompiledPatternItem],
) -> SourceResult<()> {
    // If there is no sink, we purposefully don't bother allocating
    // a set for the used keys.
    let mut sink = None;
    let mut used = has_sink.then(HashSet::new);

    for p in tuple {
        match p {
            CompiledPatternItem::Placeholder(_) => {}
            CompiledPatternItem::Simple(span, local, name) => {
                let Value::Str(key) = vm.read(*name) else {
                    unreachable!("malformed string id");
                };

                let v = dict.get(key.as_str()).at(*span)?;

                let access = vm.read(*local);
                let location = access.write(*span, vm, engine)?;

                *location = v.clone();

                used.as_mut().map(|u| u.insert(key.clone()));
            }
            CompiledPatternItem::Nested(span, _) => {
                bail!(*span, "cannot destructure unnamed pattern from dictionary");
            }
            CompiledPatternItem::Spread(span, local) => sink = Some((*span, Some(local))),
            CompiledPatternItem::SpreadDiscard(span) => sink = Some((*span, None)),
            CompiledPatternItem::Named(span, local, name) => {
                let Value::Str(key) = vm.read(*name) else {
                    unreachable!("malformed string id");
                };

                let v = dict.get(key.as_str()).at(*span)?;

                let access = vm.read(*local);
                let location = access.write(*span, vm, engine)?;
                *location = v.clone();

                used.as_mut().map(|u| u.insert(key.clone()));
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

            let access = vm.read(*local);
            let location = access.write(span, vm, engine)?;

            *location = Value::Dict(sink);
        }
    }

    Ok(())
}
