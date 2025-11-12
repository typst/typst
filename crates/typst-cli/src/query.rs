use comemo::Track;
use ecow::{EcoString, eco_format};
use typst::World;
use typst::diag::{HintedStrResult, StrResult, Warned, bail};
use typst::engine::Sink;
use typst::foundations::{Content, IntoValue, LocatableSelector, Scope};
use typst::introspection::Introspector;
use typst::layout::PagedDocument;
use typst::syntax::{Span, SyntaxMode};
use typst_eval::eval_string;
use typst_html::HtmlDocument;

use crate::args::{QueryCommand, Target};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

/// Execute a query command.
pub fn query(command: &'static QueryCommand) -> HintedStrResult<()> {
    let mut world =
        SystemWorld::new(Some(&command.input), &command.world, &command.process)?;

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let Warned { output, warnings } = match command.target {
        Target::Paged => typst::compile::<PagedDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
        Target::Html => typst::compile::<HtmlDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
    };

    match output {
        // Retrieve and print query results.
        Ok(introspector) => {
            let data = retrieve(&world, command, &introspector)?;
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
fn retrieve(
    world: &dyn World,
    command: &QueryCommand,
    introspector: &Introspector,
) -> HintedStrResult<Vec<Content>> {
    let selector = eval_string(
        &typst::ROUTINES,
        world.track(),
        // TODO: propagate warnings
        Sink::new().track_mut(),
        &command.selector,
        Span::detached(),
        SyntaxMode::Code,
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

    Ok(introspector.query(&selector.0).into_iter().collect::<Vec<_>>())
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
        crate::serialize(value, command.format, command.pretty)
    } else {
        crate::serialize(&mapped, command.format, command.pretty)
    }
}
