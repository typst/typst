use std::io::{Write, stdout};

use comemo::Track;
use ecow::{EcoVec, eco_format};
use typst::diag::{HintedStrResult, SourceResult, Warned, bail};
use typst::foundations::{Context, Output, Scope, StyleChain, Value};
use typst::routines::SpanMode;
use typst::syntax::{Span, SyntaxMode};
use typst::{World, engine::Sink, introspection::Introspector};
use typst_bundle::Bundle;
use typst_eval::eval_string;
use typst_html::HtmlDocument;
use typst_layout::PagedDocument;

use crate::args::{EvalCommand, EvalSerializationFormat, SerializationFormat, Target};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

fn print_eval_result(
    value: &Value,
    format: EvalSerializationFormat,
    pretty: bool,
) -> HintedStrResult<()> {
    match format {
        EvalSerializationFormat::Json => {
            println!("{}", crate::serialize(&value, SerializationFormat::Json, pretty)?);
        }
        EvalSerializationFormat::Yaml => {
            println!("{}", crate::serialize(&value, SerializationFormat::Yaml, pretty)?);
        }
        EvalSerializationFormat::Raw => match value {
            Value::Str(s) => {
                println!("{s}");
            }
            Value::Bytes(bytes) => {
                stdout().lock().write_all(bytes).map_err(|err| {
                    eco_format!("failed to write eval result to stdout ({err})")
                })?;
            }
            _ => bail!(
                "invalid eval result type: {}", &value.ty();
                hint: "--format=raw allows only string and bytes as result types"
            ),
        },
    };

    Ok(())
}

/// Execute a query command.
pub fn eval(command: &'static EvalCommand) -> HintedStrResult<()> {
    let mut world =
        SystemWorld::new(command.r#in.as_ref(), &command.world, &command.process)?;

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    // Compile the main file and get the introspector.
    let Warned { output, mut warnings } = match command.target {
        Target::Paged => typst::compile::<PagedDocument>(&world)
            .map(|result| result.map(|output| Box::new(output) as Box<dyn Output>)),
        Target::Html => typst::compile::<HtmlDocument>(&world)
            .map(|result| result.map(|output| Box::new(output) as Box<dyn Output>)),
        Target::Bundle => typst::compile::<Bundle>(&world)
            .map(|result| result.map(|output| Box::new(output) as Box<dyn Output>)),
    };

    let errors = match output {
        // Retrieve and print evaluation results.
        Ok(output) => {
            let mut sink = Sink::new();
            let eval_result = evaluate_expression(
                command.expression.clone(),
                &mut sink,
                &world,
                output.introspector(),
            );
            // Collect additional warnings from evaluating the expression.
            warnings.extend(sink.warnings());
            match eval_result {
                Err(errors) => errors,
                Ok(value) => {
                    print_eval_result(&value, command.format, command.pretty)?;
                    EcoVec::new()
                }
            }
        }
        // Print diagnostics.
        Err(errors) => {
            set_failed();
            errors
        }
    };

    print_diagnostics(&world, &errors, &warnings, command.process.diagnostic_format)
        .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;

    Ok(())
}

/// Evaluates the expression with code syntax mode and no scope.
fn evaluate_expression(
    expression: String,
    sink: &mut Sink,
    world: &dyn World,
    introspector: &dyn Introspector,
) -> SourceResult<Value> {
    let library = world.library();
    eval_string(
        world.track(),
        library,
        sink.track_mut(),
        introspector.track(),
        Context::new(None, Some(StyleChain::new(&library.styles))).track(),
        &expression,
        SpanMode::Uniform(Span::detached()),
        SyntaxMode::Code,
        Scope::default(),
    )
}
