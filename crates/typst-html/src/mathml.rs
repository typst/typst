use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::Locator;
use typst_library::routines::{Arenas, RealizationKind};

/// Produce MathML nodes from content.
pub fn show_equation(
    content: &Content,
    engine: &mut Engine,
    styles: StyleChain,
) -> SourceResult<Content> {
    // TODO: I recall reading in the codebase that the locator is not used by
    // HTML export, so it's fine to just fabricate one. Even if we can
    // fabricate one, does the same one need to be used throughout? For now, to
    // keep the function calls a bit leaner, I've not done this.
    let mut locator = Locator::root().split();
    let arenas = Arenas::default();
    let children = (engine.routines.realize)(
        RealizationKind::Math,
        engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let mut output = Vec::new();
    for (child, styles) in children {
        handle(child, engine, styles, &mut output)?;
    }

    Ok(Content::sequence(output))
}

fn handle(
    _elem: &Content,
    _engine: &mut Engine,
    _styles: StyleChain,
    _output: &mut [Content],
) -> SourceResult<()> {
    Ok(())
}
