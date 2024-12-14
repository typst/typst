use typst::foundations::{Label, Selector, Value};
use typst::layout::PagedDocument;
use typst::syntax::{ast, LinkedNode, Side, Source, Span};
use typst::utils::PicoStr;

use crate::utils::globals;
use crate::{
    analyze_expr, analyze_import, deref_target, named_items, DerefTarget, IdeWorld,
    NamedItem,
};

/// A definition of some item.
#[derive(Debug, Clone)]
pub enum Definition {
    /// The item is defined at the given span.
    Span(Span),
    /// The item is defined in the standard library.
    Std(Value),
}

/// Find the definition of the item under the cursor.
///
/// Passing a `document` (from a previous compilation) is optional, but enhances
/// the definition search. Label definitions, for instance, are only generated
/// when the document is available.
pub fn definition(
    world: &dyn IdeWorld,
    document: Option<&PagedDocument>,
    source: &Source,
    cursor: usize,
    side: Side,
) -> Option<Definition> {
    let root = LinkedNode::new(source.root());
    let leaf = root.leaf_at(cursor, side)?;

    match deref_target(leaf.clone())? {
        // Try to find a named item (defined in this file or an imported file)
        // or fall back to a standard library item.
        DerefTarget::VarAccess(node) | DerefTarget::Callee(node) => {
            let name = node.cast::<ast::Ident>()?.get().clone();
            if let Some(src) = named_items(world, node.clone(), |item: NamedItem| {
                (*item.name() == name).then(|| Definition::Span(item.span()))
            }) {
                return Some(src);
            };

            if let Some((value, _)) = analyze_expr(world, &node).first() {
                let span = match value {
                    Value::Content(content) => content.span(),
                    Value::Func(func) => func.span(),
                    _ => Span::detached(),
                };
                if !span.is_detached() && span != node.span() {
                    return Some(Definition::Span(span));
                }
            }

            if let Some(binding) = globals(world, &leaf).get(&name) {
                return Some(Definition::Std(binding.read().clone()));
            }
        }

        // Try to jump to the an imported file or package.
        DerefTarget::ImportPath(node) | DerefTarget::IncludePath(node) => {
            let Some(Value::Module(module)) = analyze_import(world, &node) else {
                return None;
            };
            let id = module.file_id()?;
            let span = Span::from_range(id, 0..0);
            return Some(Definition::Span(span));
        }

        // Try to jump to the referenced content.
        DerefTarget::Ref(node) => {
            let label = Label::new(PicoStr::intern(node.cast::<ast::Ref>()?.target()));
            let selector = Selector::Label(label);
            let elem = document?.introspector.query_first(&selector)?;
            return Some(Definition::Span(elem.span()));
        }

        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::ops::Range;

    use typst::foundations::{IntoValue, NativeElement};
    use typst::syntax::Side;
    use typst::WorldExt;

    use super::{definition, Definition};
    use crate::tests::{FilePos, TestWorld, WorldLike};

    type Response = (TestWorld, Option<Definition>);

    trait ResponseExt {
        fn must_be_at(&self, path: &str, range: Range<usize>) -> &Self;
        fn must_be_value(&self, value: impl IntoValue) -> &Self;
    }

    impl ResponseExt for Response {
        #[track_caller]
        fn must_be_at(&self, path: &str, expected: Range<usize>) -> &Self {
            match self.1 {
                Some(Definition::Span(span)) => {
                    let range = self.0.range(span);
                    assert_eq!(
                        span.id().unwrap().vpath().as_rootless_path().to_string_lossy(),
                        path
                    );
                    assert_eq!(range, Some(expected));
                }
                _ => panic!("expected span definition"),
            }
            self
        }

        #[track_caller]
        fn must_be_value(&self, expected: impl IntoValue) -> &Self {
            match &self.1 {
                Some(Definition::Std(value)) => {
                    assert_eq!(*value, expected.into_value())
                }
                _ => panic!("expected std definition"),
            }
            self
        }
    }

    #[track_caller]
    fn test(world: impl WorldLike, pos: impl FilePos, side: Side) -> Response {
        let world = world.acquire();
        let world = world.borrow();
        let doc = typst::compile(world).output.ok();
        let (source, cursor) = pos.resolve(world);
        let def = definition(world, doc.as_ref(), &source, cursor, side);
        (world.clone(), def)
    }

    #[test]
    fn test_definition_let() {
        test("#let x; #x", -2, Side::After).must_be_at("main.typ", 5..6);
        test("#let x() = {}; #x", -2, Side::After).must_be_at("main.typ", 5..6);
    }

    #[test]
    fn test_definition_field_access_function() {
        let world = TestWorld::new("#import \"other.typ\"; #other.foo")
            .with_source("other.typ", "#let foo(x) = x + 1");

        // The span is at the args here because that's what the function value's
        // span is. Not ideal, but also not too big of a big deal.
        test(&world, -2, Side::Before).must_be_at("other.typ", 8..11);
    }

    #[test]
    fn test_definition_cross_file() {
        let world = TestWorld::new("#import \"other.typ\": x; #x")
            .with_source("other.typ", "#let x = 1");
        test(&world, -2, Side::After).must_be_at("other.typ", 5..6);
    }

    #[test]
    fn test_definition_import() {
        let world = TestWorld::new("#import \"other.typ\" as o: x")
            .with_source("other.typ", "#let x = 1");
        test(&world, 14, Side::Before).must_be_at("other.typ", 0..0);
    }

    #[test]
    fn test_definition_include() {
        let world = TestWorld::new("#include \"other.typ\"")
            .with_source("other.typ", "Hello there");
        test(&world, 14, Side::Before).must_be_at("other.typ", 0..0);
    }

    #[test]
    fn test_definition_ref() {
        test("#figure[] <hi> See @hi", -2, Side::After).must_be_at("main.typ", 1..9);
    }

    #[test]
    fn test_definition_std() {
        test("#table", 1, Side::After).must_be_value(typst::model::TableElem::elem());
    }
}
