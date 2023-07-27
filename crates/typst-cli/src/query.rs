use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::{bail, Severity, SourceDiagnostic, StrResult};
use typst::eval::{eco_format, Tracer, Value};
use typst::model::{Introspector, Selector};
use typst::World;
use typst_library::meta::ProvideElem;
use typst_library::prelude::*;

use crate::args::{CompileCommand, DiagnosticFormat, QueryCommand};
use crate::world::SystemWorld;
use crate::{color_stream, set_failed};

/// Execute a compilation command.
pub fn query(command: QueryCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&CompileCommand {
        // Little hack, only 3 fields are used
        font_paths: command.font_paths.clone(),
        input: command.input.clone(),
        root: command.root.clone(),
        output: None,
        flamegraph: None,
        open: None,
        diagnostic_format: command.diagnostic_format,
        ppi: 0.0,
    })?;
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

            let mut params = Dict::new();
            params.insert("key".into(), Value::Str(command.key.clone().into()));

            let provided_metadata = introspector
                .query(&Selector::Elem(ProvideElem::func(), Some(params)))
                .iter()
                .filter_map(|c| c.field("value"))
                .collect();

            export(&provided_metadata, &command)?;

            tracing::info!("Processing succeeded in {duration:?}");

            print_diagnostics(&world, &[], &warnings, command.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            tracing::info!("Processing failed");

            print_diagnostics(&world, &errors, &warnings, command.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

fn export(metadata: &Vec<Value>, command: &QueryCommand) -> StrResult<()> {
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
                "toml" => {
                    toml::ser::to_string(&metadata[0]).map_err(|e| e.to_string())?
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
            "toml" => toml::ser::to_string(&metadata).map_err(|e| e.to_string())?,
            _ => bail!("Unknown format"),
        };
        println!("{result}");
        Ok(())
    }
}

/// Print diagnostic messages to the terminal.
fn print_diagnostics(
    world: &SystemWorld,
    errors: &[SourceDiagnostic],
    warnings: &[SourceDiagnostic],
    diagnostic_format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = match diagnostic_format {
        DiagnosticFormat::Human => color_stream(),
        DiagnosticFormat::Short => StandardStream::stderr(ColorChoice::Never),
    };

    let mut config = term::Config { tab_width: 2, ..Default::default() };
    if diagnostic_format == DiagnosticFormat::Short {
        config.display_style = term::DisplayStyle::Short;
    }

    for diagnostic in warnings.iter().chain(errors.iter()) {
        let diag = match diagnostic.severity {
            Severity::Error => Diagnostic::error(),
            Severity::Warning => Diagnostic::warning(),
        }
        .with_message(diagnostic.message.clone())
        .with_notes(
            diagnostic
                .hints
                .iter()
                .map(|e| (eco_format!("hint: {e}")).into())
                .collect(),
        )
        .with_labels(vec![Label::primary(
            diagnostic.span.id(),
            world.range(diagnostic.span),
        )]);

        term::emit(&mut w, &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in &diagnostic.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.id(), world.range(point.span)),
            ]);

            term::emit(&mut w, &config, world, &help)?;
        }
    }

    Ok(())
}
