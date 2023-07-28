use comemo::Track;
use serde::Serialize;
use std::collections::HashMap;
use typst::diag::{bail, StrResult};
use typst::eval::Value::Dyn;
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
                let provided_metadata = introspector
                    .query(&Selector::Elem(
                        ProvideElem::func(),
                        Some(Dict::from_iter([("key".into(), key.clone().into_value())])),
                    ))
                    .iter()
                    .filter_map(|c| c.field("value"))
                    .collect::<Vec<_>>();
                export(&provided_metadata, &command)?;
            }

            if let Some(selector) = &command.selector {
                let selected_metadata = introspector
                    .query(&make_selector(&selector, &world)?)
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

fn make_selector(selector_description: &str, world: &dyn World) -> StrResult<Selector> {
    let evaluated_selector = eval_string(
        world.track(),
        &format!("selector({selector_description})"),
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
    .map_err(|_| "Error evaluating the selector string.")?;

    let Dyn(selector) = evaluated_selector else {
        bail!("Parsing of selector string not successfull.")
    };

    Ok(selector.downcast::<Selector>().unwrap().to_owned())
}

fn export<T: Serialize>(data: &[T], command: &QueryCommand) -> StrResult<()> {
    if command.one && data.len() != 1 {
        bail!("One piece of metadata expected, but {} found.", data.len())
    }

    let result = match (command.format.as_str(), command.one) {
        ("json", true) => serde_json::to_string(&data[0]).map_err(|e| e.to_string())?,
        ("yaml", true) => serde_yaml::to_string(&data[0]).map_err(|e| e.to_string())?,
        ("json", false) => serde_json::to_string(&data).map_err(|e| e.to_string())?,
        ("yaml", false) => serde_yaml::to_string(&data).map_err(|e| e.to_string())?,
        _ => bail!("Unknown format"),
    };

    println!("{result}");
    Ok(())
}
