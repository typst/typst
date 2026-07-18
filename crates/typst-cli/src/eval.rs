use std::sync::LazyLock;

use comemo::Track;
use ecow::eco_format;
use typst::Library;
use typst::diag::{FileResult, HintedStrResult, SourceResult, Warned};
use typst::foundations::{
    Bytes, Context, Datetime, Duration, Output, Scope, StyleChain, Value,
};
use typst::routines::SpanMode;
use typst::syntax::{
    FileId, RangeMapper, RootedPath, Source, Span, SyntaxMode, VirtualPath, VirtualRoot,
};
use typst::text::{Font, FontBook};
use typst::{
    World,
    engine::{Route, Sink},
    introspection::Introspector,
};
use typst_bundle::Bundle;
use typst_eval::eval_string;
use typst_html::HtmlDocument;
use typst_kit::diagnostics::DiagnosticWorld;
use typst_layout::PagedDocument;
use typst_utils::LazyHash;

use crate::args::{EvalCommand, Target};
use crate::compile::print_diagnostics;
use crate::set_failed;
use crate::world::SystemWorld;

/// Evaluate an input expression, potentially as a query over an existing
/// document.
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

    match output {
        // The target compiled successfully, continue with evaluating the input
        // expression.
        Ok(output) => {
            let expr_world = ExpressionWorld {
                world,
                expression: Bytes::from_string(&*command.expression),
            };
            let mut sink = Sink::new();
            let eval_result = evaluate_expression(
                &command.expression,
                &mut sink,
                &expr_world,
                output.introspector(),
            );
            let errors = match &eval_result {
                Err(errors) => {
                    set_failed();
                    errors.as_slice()
                }
                Ok(value) => {
                    let serialized =
                        crate::serialize(value, command.format, command.pretty)?;
                    println!("{serialized}");
                    &[]
                }
            };
            // Collect additional warnings from evaluating the expression.
            warnings.extend(sink.warnings());

            print_diagnostics(
                &expr_world,
                errors,
                &warnings,
                command.process.diagnostic_format,
            )
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }

        // The target failed, print its diagnostics.
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

/// Evaluate the input expression in [`SyntaxMode::Code`] and with no scope.
fn evaluate_expression(
    expression: &str,
    sink: &mut Sink,
    world: &dyn World,
    introspector: &dyn Introspector,
) -> SourceResult<Value> {
    // Map spans to ranges in the string.
    let spans = SpanMode::Mapped {
        id: *EXPRESSION_ID,
        mapper: &RangeMapper::new(Some(0..expression.len())).unwrap(),
        mapper_error_span: Span::detached(),
    };
    let library = world.library();
    eval_string(
        world.track(),
        library,
        sink.track_mut(),
        introspector.track(),
        Route::default().track(),
        Context::new(None, Some(StyleChain::new(&library.styles))).track(),
        expression,
        spans,
        SyntaxMode::Code,
        Scope::default(),
    )
}

/// Static [`FileId`] for an input expression. This allows giving accurate
/// ranges to diagnostics when evaluating the expression.
static EXPRESSION_ID: LazyLock<FileId> = LazyLock::new(|| {
    FileId::unique(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("<input-expression>").unwrap(),
    ))
});

/// A world wrapper that allows us to print accurate diagnostics for an input
/// expression.
struct ExpressionWorld {
    world: SystemWorld,
    /// We use `Bytes`, not `&'static str` to avoid hashing in `World::file`.
    expression: Bytes,
}

impl DiagnosticWorld for ExpressionWorld {
    fn name(&self, id: FileId) -> String {
        if id == *EXPRESSION_ID {
            "<input-expression>".into()
        } else {
            self.world.name(id)
        }
    }
}

impl World for ExpressionWorld {
    fn library(&self) -> &LazyHash<Library> {
        self.world.library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.world.book()
    }

    fn main(&self) -> FileId {
        self.world.main()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.world.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == *EXPRESSION_ID {
            Ok(self.expression.clone())
        } else {
            self.world.file(id)
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.world.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.world.today(offset)
    }
}
