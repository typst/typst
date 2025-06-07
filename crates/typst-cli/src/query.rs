use comemo::Track;
use ecow::{eco_format, EcoString};
use serde::Serialize;
use typst::diag::{bail, HintedStrResult, StrResult, Warned};
use typst::engine::Sink;
use typst::foundations::{Content, IntoValue, LocatableSelector, Scope};
use typst::html::HtmlDocument;
use typst::layout::PagedDocument;
use typst::syntax::Span;
use typst::{Document, World};
use typst_eval::{eval_string, EvalMode};

use crate::args::{QueryCommand, SerializationFormat, Target};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

/// Execute a query command.
pub fn query(command: &QueryCommand) -> HintedStrResult<()> {
    let mut world = SystemWorld::new(&command.input, &command.world, &command.process)?;

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let Warned { output, warnings } = match command.target {
        Target::Paged => typst::compile::<PagedDocument>(&world)
            .map(|output| output.map(|document| retrieve(&world, command, &document))),
        Target::Html => typst::compile::<HtmlDocument>(&world)
            .map(|output| output.map(|document| retrieve(&world, command, &document))),
    };

    match output {
        // Retrieve and print query results.
        Ok(data) => {
            let data = data?;
            let serialized = format(data, command)?;
            println!("{serialized}");
            print_diagnostics(&world, &[], &warnings, command.process.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            print_diagnostics(
                &world,
                &errors,
                &warnings,
                command.process.diagnostic_format,
            )
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }
    }

    Ok(())
}

/// Retrieve the matches for the selector.
fn retrieve<D: Document>(
    world: &dyn World,
    command: &QueryCommand,
    document: &D,
) -> HintedStrResult<Vec<Content>> {
    let selector = eval_string(
        &typst::ROUTINES,
        world.track(),
        // TODO: propagate warnings
        Sink::new().track_mut(),
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

    Ok(document
        .introspector()
        .query(&selector.0)
        .into_iter()
        .collect::<Vec<_>>())
}

/// Format the query result in the output format.
fn format(elements: Vec<Content>, command: &QueryCommand) -> StrResult<String> {
    if command.one && elements.len() != 1 {
        bail!("expected exactly one element, found {}", elements.len());
    }

    let mapped: Vec<_> = elements
        .into_iter()
        .filter_map(|c| match &command.field {
            Some(field) => c.get_by_name(field).ok(),
            _ => Some(c.into_value()),
        })
        .collect();

    if command.one {
        let Some(value) = mapped.first() else {
            bail!("no such field found for element");
        };
        serialize(value, command.format, command.pretty)
    } else {
        serialize(&mapped, command.format, command.pretty)
    }
}

/// Serialize data to the output format.
fn serialize(
    data: &impl Serialize,
    format: SerializationFormat,
    pretty: bool,
) -> StrResult<String> {
    match format {
        SerializationFormat::Json => {
            if pretty {
                serde_json::to_string_pretty(data).map_err(|e| eco_format!("{e}"))
            } else {
                serde_json::to_string(data).map_err(|e| eco_format!("{e}"))
            }
        }
        SerializationFormat::Yaml => {
            serde_yaml::to_string(data).map_err(|e| eco_format!("{e}"))
        }
    }
}
