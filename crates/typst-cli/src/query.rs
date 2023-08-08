use comemo::Track;
use serde::Serialize;
use typst::diag::{bail, StrResult};
use typst::eval::{eval_string, EvalMode, Tracer};
use typst::model::Introspector;
use typst::World;
use typst_library::prelude::*;

use crate::args::{QueryCommand, SerializationFormat};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

/// Execute a query command.
pub fn query(command: QueryCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&command.common)?;
    tracing::info!("Starting querying");

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let mut tracer = Tracer::default();
    let result = typst::compile(&world, &mut tracer);
    let warnings = tracer.warnings();

    match result {
        // Retrieve and print query results.
        Ok(document) => {
            let data = retrieve(&world, &command, &document)?;
            let serialized = format(data, &command)?;
            println!("{serialized}");
            print_diagnostics(&world, &[], &warnings, command.common.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
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

/// Retrieve the matches for the selector.
fn retrieve(
    world: &dyn World,
    command: &QueryCommand,
    document: &Document,
) -> StrResult<Vec<Content>> {
    let selector = eval_string(
        world.track(),
        &command.selector,
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
    .map_err(|errors| {
        let mut message = EcoString::from("failed to evaluate selector");
        for (i, error) in errors.into_iter().enumerate() {
            message.push_str(if i == 0 { ": " } else { ", " });
            message.push_str(&error.message);
        }
        message
    })?
    .cast::<LocatableSelector>()?;

    Ok(Introspector::new(&document.pages)
        .query(&selector.0)
        .into_iter()
        .map(|x| x.into_inner())
        .collect::<Vec<_>>())
}

/// Format the query result in the output format.
fn format(elements: Vec<Content>, command: &QueryCommand) -> StrResult<String> {
    if command.one && elements.len() != 1 {
        bail!("expected exactly one element, found {}", elements.len())
    }

    let mapped: Vec<_> = elements
        .into_iter()
        .filter_map(|c| match &command.field {
            Some(field) => c.field(field),
            _ => Some(c.into_value()),
        })
        .collect();

    if command.one {
        serialize(&mapped[0], command.format)
    } else {
        serialize(&mapped, command.format)
    }
}

/// Serialize data to the output format.
fn serialize(data: &impl Serialize, format: SerializationFormat) -> StrResult<String> {
    match format {
        SerializationFormat::Json => {
            serde_json::to_string_pretty(data).map_err(|e| eco_format!("{e}"))
        }
        SerializationFormat::Yaml => {
            serde_yaml::to_string(&data).map_err(|e| eco_format!("{e}"))
        }
    }
}
