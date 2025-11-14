use crate::args::{EvalCommand, FileInput, StringInput, SyntaxMode, Target};
use crate::world::{SystemWorld, decode_utf8, read_from_stdin};
use crate::{compile::print_diagnostics, set_failed};
use comemo::Track;
use ecow::{EcoString, eco_format};
use typst::diag::{HintedStrResult, SourceResult, Warned};
use typst::foundations::{Binding, Context, Scope, StyleChain, Value};
use typst::{World, introspection::Introspector, layout::PagedDocument};
use typst::{engine::Sink, syntax::Span};
use typst_eval::eval_string;
use typst_html::HtmlDocument;

/// Execute a query command.
pub fn eval(command: &'static EvalCommand) -> HintedStrResult<()> {
    let mut world = SystemWorld::new(
        command.r#in.clone().map(FileInput::Path).as_ref(),
        &command.world,
        &command.process,
    )?;

    // Reset everything and ensure that the main file is present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    // Compile the main file and get the introspector.
    let Warned { output, mut warnings } = match command.target {
        Target::Paged => typst::compile::<PagedDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
        Target::Html => typst::compile::<HtmlDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
    };

    match output {
        // Retrieve and print evaluation results.
        Ok(introspector) => {
            let mut sink = Sink::new();
            let scope = evaluate_scope(&command.scope, &mut sink, &world, &introspector)?;
            let statement = match &command.statement {
                StringInput::Stdin => read_statement_from_stdin()?,
                StringInput::String(statement) => statement.clone(),
            };
            let eval_result = evaluate_statement(
                statement,
                command.mode,
                scope,
                &mut sink,
                &world,
                &introspector,
            );
            let errors = match &eval_result {
                Err(errors) => errors.as_slice(),
                Ok(value) => {
                    let serialized =
                        crate::serialize(value, command.format, command.pretty)?;
                    println!("{serialized}");
                    &[]
                }
            };
            // Collect additional warnings from code evaluations
            warnings.extend(sink.warnings());

            print_diagnostics(
                &world,
                errors,
                &warnings,
                command.process.diagnostic_format,
            )
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

/// Evaluates the scope with values interpreted as Typst code.
fn evaluate_scope(
    key_value_pairs: &[(String, String)],
    sink: &mut Sink,
    world: &dyn World,
    introspector: &Introspector,
) -> HintedStrResult<Scope> {
    let mut scope = Scope::new();

    for (key, value) in key_value_pairs {
        let mut local_sink = Sink::new();
        let value = evaluate_statement(
            value.clone(),
            SyntaxMode::Code,
            Scope::default(),
            &mut local_sink,
            world,
            introspector,
        )
        .map_err(|errors| {
            let mut message = EcoString::from(format!(r#"scope value for "{key}""#));
            for (i, error) in errors.into_iter().enumerate() {
                message.push_str(if i == 0 { ": " } else { ", " });
                message.push_str(&error.message);
            }
            message
        })?;

        // Propagate warnings from code evaluations
        for mut warning in local_sink.warnings() {
            warning.message =
                eco_format!(r#"scope value for "{key}": {}"#, warning.message);
            sink.warn(warning);
        }

        // Bind the evaluated value to the scope aggregated so far
        scope.bind(key.into(), Binding::detached(value));
    }

    Ok(scope)
}

/// Evaluates the statement with the given mode and scope.
fn evaluate_statement(
    statement: String,
    mode: SyntaxMode,
    scope: Scope,
    sink: &mut Sink,
    world: &dyn World,
    introspector: &Introspector,
) -> SourceResult<Value> {
    eval_string(
        &typst::ROUTINES,
        world.track(),
        sink.track_mut(),
        introspector.track(),
        Context::new(None, Some(StyleChain::new(&world.library().styles))).track(),
        &statement,
        Span::detached(),
        match mode {
            SyntaxMode::Code => typst::syntax::SyntaxMode::Code,
            SyntaxMode::Markup => typst::syntax::SyntaxMode::Markup,
            SyntaxMode::Math => typst::syntax::SyntaxMode::Math,
        },
        scope,
    )
}

/// Reads a statement from stdin, decoding it from UTF-8.
fn read_statement_from_stdin() -> HintedStrResult<String> {
    let result = read_from_stdin()?;
    let statement = decode_utf8(&result)?;
    Ok(statement.into())
}
