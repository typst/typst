use crate::{
    args::{EvalCommand, FileInput, StringInput, SyntaxMode, Target},
    compile::print_diagnostics,
    set_failed,
    world::SystemWorld,
};
use comemo::Track;
use ecow::{EcoString, eco_format};
use typst::{
    World,
    diag::{HintedStrResult, SourceResult, Warned},
    engine::Sink,
    foundations::{Binding, Context, Scope, StyleChain, Value},
    introspection::Introspector,
    layout::PagedDocument,
    syntax::Span,
};
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
    let Warned { output, warnings } = match /* command.target */ Target::Paged {
        Target::Paged => typst::compile::<PagedDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
        Target::Html => typst::compile::<HtmlDocument>(&world)
            .map(|output| output.map(|document| document.introspector)),
    };

    match output {
        // Retrieve and print query results.
        Ok(introspector) => {
            let scope = evaluate_scope(&command.scope, &world, &introspector)?;
            let statement = match &command.statement {
                StringInput::Stdin => read_statement_from_stdin().map_err(|err| err.to_string())?,
                StringInput::String(statement) => statement.clone(),
            };
            let eval_result =
                evaluate_statement(statement, command.mode, scope, &world, &introspector);
            let errors = match &eval_result {
                Err(errors) => errors.as_slice(),
                Ok(value) => {
                    let serialized =
                        crate::serialize(value, command.format, command.pretty)?;
                    println!("{serialized}");
                    &[]
                }
            };
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
    world: &dyn World,
    introspector: &Introspector,
) -> HintedStrResult<Scope> {
    let mut scope = Scope::new();

    for (key, value) in key_value_pairs {
        let value = evaluate_statement(
            value.clone(),
            SyntaxMode::Code,
            Scope::default(),
            world,
            introspector,
        )
        .map_err(|errors| {
            let mut message =
                EcoString::from(format!("failure in scope key `{key}` evaluation"));
            for (i, error) in errors.into_iter().enumerate() {
                message.push_str(if i == 0 { ": " } else { ", " });
                message.push_str(&error.message);
            }
            message
        })?;

        scope.bind(key.into(), Binding::detached(value));
    }

    Ok(scope)
}

/// Evaluates the statement with the given mode and scope.
fn evaluate_statement(
    statement: String,
    mode: SyntaxMode,
    scope: Scope,
    world: &dyn World,
    introspector: &Introspector,
) -> SourceResult<Value> {
    // Pretty much shamelessly copied from typst-eval::eval_string
    let root = match mode {
        SyntaxMode::Code => parse_code(&statement),
        SyntaxMode::Markup => parse(&statement),
        SyntaxMode::Math => parse_math(&statement),
    };

    // Check for well-formedness.
    let errors = root.errors();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(Into::into).collect());
    }

    // Prepare the engine.
    let introspector = introspector.clone();
    let traced = Traced::default();
    let mut binding = Sink::new();
    let engine = Engine {
        routines: &typst::ROUTINES,
        world: world.track(),
        introspector: introspector.track(),
        traced: traced.track(),
        sink: binding.track_mut(),
        route: Route::default(),
    };

    // Prepare VM.
    let context = Context::new(None, Some(StyleChain::new(&world.library().styles)));
    let scopes = Scopes::new(Some(world.library()));
    let mut vm = Vm::new(engine, context.track(), scopes, root.span());
    vm.scopes.scopes.push(scope);

    // Evaluate the code.
    let output = match mode {
        SyntaxMode::Code => root.cast::<ast::Code>().unwrap().eval(&mut vm)?,
        SyntaxMode::Markup => {
            Value::Content(root.cast::<ast::Markup>().unwrap().eval(&mut vm)?)
        }
        SyntaxMode::Math => Value::Content(
            EquationElem::new(root.cast::<ast::Math>().unwrap().eval(&mut vm)?)
                .with_block(false)
                .pack(),
        ),
    };

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    Ok(output)
}

fn read_statement_from_stdin() -> FileResult<String> {
    let mut buf = Vec::new();
    let result = io::stdin().read_to_end(&mut buf);
    match result {
        Ok(_) => (),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => (),
        Err(err) => return Err(FileError::from_io(err, Path::new("<stdin>"))),
    }
    let statement = std::str::from_utf8(&buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&buf))?;
    Ok(statement.to_string())
}
