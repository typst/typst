use comemo::Track;
use serde::Serialize;
use std::collections::HashMap;
use typst::diag::{bail, StrResult};
use typst::eval::{eval_string, EvalMode, Tracer, Value};
use typst::model::{Introspector, Selector};
use typst::World;
use typst_library::meta::ProvideElem;
use typst_library::prelude::*;

use crate::args::QueryCommand;
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

#[derive(Serialize)]
pub struct SelectedElement {
    #[serde(rename = "type")]
    typename: String,
    attributes: HashMap<EcoString, Value>,
}

/// Execute a compilation command.
pub fn query(command: QueryCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&command.common)?;
    tracing::info!("Starting querying");

    let start = std::time::Instant::now();
    // Reset everything and ensure that the main file is still present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let mut tracer = Tracer::default();
    let result = typst::compile(&world, &mut tracer);
    let duration = start.elapsed();
    let warnings = tracer.warnings();

    match result {
        // Print metadata
        Ok(document) => {
            let introspector = Introspector::new(&document.pages);

            if let Some(key) = &command.key {
                let mut params = Dict::new();
                params.insert("key".into(), Value::Str(key.clone().into()));
                let provided_metadata = introspector
                    .query(&Selector::Elem(ProvideElem::func(), Some(params)))
                    .iter()
                    .filter_map(|c| c.field("value"))
                    .collect::<Vec<_>>();
                export(&provided_metadata, &command)?;
            } else if let Some(selector) = &command.selector {
                let dworld: &dyn World = &world;
                let eval = eval_string(
                    dworld.track(),
                    selector,
                    Span::detached(),
                    EvalMode::Code,
                    Scope::default(),
                )
                .map_err(|_| "Error on eval")?;
                let selected_metadata = introspector
                    .query(&make_selector(&eval)?)
                    .into_iter()
                    .map(|x| SelectedElement {
                        typename: x.func().name().into(),
                        attributes: x
                            .clone()
                            .into_inner()
                            .fields()
                            .map(|(k, v)| (k.clone(), v))
                            .collect(),
                    })
                    .collect::<Vec<_>>();
                export(&selected_metadata, &command)?;
            } else {
                bail!("Should not happen");
            };

            tracing::info!("Processing succeeded in {duration:?}");

            print_diagnostics(&world, &[], &warnings, command.common.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            tracing::info!("Processing failed");

            print_diagnostics(
                &world,
                &errors,
                &warnings,
                command.common.diagnostic_format,
            )
            .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

fn make_selector(v: &Value) -> StrResult<Selector> {
    Ok(match v {
        Value::Dyn(dyn_value) => {
            if dyn_value.is::<Selector>() {
                let selector: &Selector = dyn_value.downcast::<Selector>().unwrap();
                selector.to_owned()
            } else {
                bail!("Cannot cast dynamic {} to selector", v.type_name())
            }
        }
        Value::Func(func) => Selector::Elem(func.element().unwrap().to_owned(), None),
        Value::Label(label) => Selector::Label(label.to_owned()),
        _ => bail!("Cannot cast static {} to selector", v.type_name()),
    })
}

fn export<T: serde::ser::Serialize>(
    metadata: &[T],
    command: &QueryCommand,
) -> StrResult<()> {
    if command.one {
        if metadata.len() != 1 {
            Err(format!("One piece of metadata expected, but {} found.", metadata.len())
                .into())
        } else {
            let result = match command.format.as_str() {
                "json" => {
                    serde_json::to_string(&metadata[0]).map_err(|e| e.to_string())?
                }
                "yaml" => {
                    serde_yaml::to_string(&metadata[0]).map_err(|e| e.to_string())?
                }
                _ => bail!("Unknown format"),
            };
            println!("{result}");
            Ok(())
        }
    } else {
        let result = match command.format.as_str() {
            "json" => serde_json::to_string(&metadata).map_err(|e| e.to_string())?,
            "yaml" => serde_yaml::to_string(&metadata).map_err(|e| e.to_string())?,
            _ => bail!("Unknown format"),
        };
        println!("{result}");
        Ok(())
    }
}
