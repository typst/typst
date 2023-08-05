use std::collections::HashMap;
use std::time::Instant;

use comemo::Track;
use serde::Serialize;
use typst::diag::{bail, StrResult};
use typst::eval::Value::Dyn;
use typst::eval::{eval_string, EvalMode, Tracer, Value};
use typst::model::{Introspector, Selector};
use typst::World;
use typst_library::meta::ProvideElem;
use typst_library::prelude::*;

use crate::args::{OutputFormat, QueryCommand};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

#[derive(Serialize)]
pub struct SelectedElement {
    #[serde(rename = "type")]
    typename: String,
    attributes: HashMap<EcoString, Value>,
}

/// Execute a query command.
pub fn query(command: QueryCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&command.common)?;
    tracing::info!("Starting querying");

    let start = Instant::now();
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
                let provided_metadata = introspector
                    .query(&keyvalue_selector(key))
                    .iter()
                    .filter_map(|c| c.field("value"))
                    .collect::<Vec<_>>();
                format(&provided_metadata, &command)?;
            }

            if let Some(selector) = &command.selector {
                let selected_metadata = introspector
                    .query(&generic_selector(selector, &world)?)
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
                format(&selected_metadata, &command)?;
            }

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

fn generic_selector(description: &str, world: &dyn World) -> StrResult<Selector> {
    let evaluated_selector = eval_string(
        world.track(),
        &format!("selector({description})"),
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
    .map_err(|_| "Error evaluating the selector string.")?;

    let Dyn(selector) = evaluated_selector else {
        bail!("Parsing of selector string not successfully.")
    };

    Ok(selector.downcast::<Selector>().unwrap().to_owned())
}

fn keyvalue_selector(key: &str) -> Selector {
    let mut bounds = Dict::new();
    bounds.insert("key".into(), key.into_value());

    Selector::Elem(ProvideElem::func(), Some(bounds))
}

fn format<T: Serialize>(data: &[T], command: &QueryCommand) -> StrResult<()> {
    if command.one && data.len() != 1 {
        bail!("One piece of metadata expected, but {} found.", data.len())
    }

    let result = match (&command.format, command.one) {
        (OutputFormat::Json, true) => {
            serde_json::to_string(&data[0]).map_err(|e| e.to_string())?
        }
        (OutputFormat::Yaml, true) => {
            serde_yaml::to_string(&data[0]).map_err(|e| e.to_string())?
        }
        (OutputFormat::Json, false) => {
            serde_json::to_string(&data).map_err(|e| e.to_string())?
        }
        (OutputFormat::Yaml, false) => {
            serde_yaml::to_string(&data).map_err(|e| e.to_string())?
        }
    };

    println!("{result}");
    Ok(())
}
